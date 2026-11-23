//! Background worker: backfill `media_translations.embedding`, then rebuild
//! the cached per-profile recommendation lists. Run with `--once` for a
//! one-shot pass (e.g. from cron) or leave it running for periodic refresh.

use std::{error::Error, time::Instant};

use chrono::{Duration as ChronoDuration, Utc};
use clap::Parser;
use diesel::PgConnection;
use dionysus_db::{
  models::NewRecommendation,
  recommendations::{
    ScoredMedia, for_profile_by_cooccurrence, friends_watched, latest_watched_media,
    list_profile_ids, nearest_to_embedding, pending_translations, positive_embeddings,
    profile_owner, replace_recommendations, set_translation_embedding, similar_by_cooccurrence,
    similar_by_embedding,
  },
};
use dionysus_embeddings::{Embedder, HttpEmbedder};
use pgvector::Vector as PgVector;
use uuid::Uuid;

#[derive(Parser)]
#[command(
  version,
  about = "Backfill embeddings and refresh recommendation caches"
)]
struct Args {
  /// Run one pass and exit instead of looping.
  #[arg(long)]
  once: bool,
  /// Translations embedded per batch.
  #[arg(long, default_value_t = 64)]
  batch_size: usize,
  /// Suggestions kept per profile and reason.
  #[arg(long, default_value_t = 50)]
  limit: i64,
}

type WorkerError = Box<dyn Error + Send + Sync>;

async fn backfill_embeddings(
  conn: &mut PgConnection,
  embedder: &dyn Embedder,
  batch_size: usize,
) -> Result<usize, WorkerError> {
  let mut total = 0;
  loop {
    let pending = pending_translations(conn, batch_size as i64)?;
    if pending.is_empty() {
      break;
    }
    let texts: Vec<String> = pending.iter().map(|row| row.embed_text()).collect();
    let vectors = embedder.embed(&texts).await?;
    for (row, vector) in pending.iter().zip(vectors) {
      set_translation_embedding(conn, row.id, PgVector::from(vector))?;
    }
    total += pending.len();
    if pending.len() < batch_size {
      break;
    }
  }
  Ok(total)
}

fn to_new<'a>(
  rows: Vec<ScoredMedia>,
  profile_id: Uuid,
  reason: &'a str,
  algorithm: &'a str,
  source: Option<Uuid>,
  ttl: ChronoDuration,
) -> Vec<NewRecommendation<'a>> {
  rows
    .into_iter()
    .map(|row| NewRecommendation {
      profile_id,
      media_id: row.media_id,
      score: row.score,
      reason,
      source_media_id: source,
      algorithm,
      expires_at: Some(Utc::now() + ttl),
    })
    .collect()
}

fn refresh_caches(
  conn: &mut PgConnection,
  embedder: &dyn Embedder,
  locale: &str,
  limit: i64,
  ttl: ChronoDuration,
) -> Result<(), WorkerError> {
  for profile_id in list_profile_ids(conn)? {
    // "For you": average the embeddings of watched/saved titles, then
    // nearest-neighbour the centroid. Falls back to co-occurrence when the
    // profile has no embedded titles yet.
    let positive = positive_embeddings(conn, profile_id, locale)?;
    let mut for_you = Vec::new();
    if !positive.is_empty() {
      let dimensions = embedder.dimensions();
      let mut centroid = vec![0.0_f32; dimensions];
      let mut counted = 0_usize;
      for row in &positive {
        let vector = row.embedding.to_vec();
        if vector.len() != dimensions {
          tracing::warn!(media_id = %row.media_id, "embedding has the wrong dimension, skipping");
          continue;
        }
        for (sum, value) in centroid.iter_mut().zip(vector) {
          *sum += value;
        }
        counted += 1;
      }
      if counted > 0 {
        for value in &mut centroid {
          *value /= counted as f32;
        }
        let nearest =
          nearest_to_embedding(conn, &PgVector::from(centroid), locale, profile_id, limit)?;
        for_you = to_new(
          nearest,
          profile_id,
          "for_you",
          "embedding_centroid",
          None,
          ttl,
        );
      }
    }
    if for_you.is_empty() {
      let rows = for_profile_by_cooccurrence(conn, profile_id, limit)?;
      for_you = to_new(rows, profile_id, "for_you", "cooccurrence", None, ttl);
    }
    replace_recommendations(conn, profile_id, "for_you", &for_you)?;

    // "Because you watched": anchor on the most recent title, using content
    // similarity first and co-occurrence as the fallback.
    if let Some(seed) = latest_watched_media(conn, profile_id)? {
      let mut rows = similar_by_embedding(conn, seed, locale, limit)?;
      let algorithm = if rows.is_empty() {
        rows = similar_by_cooccurrence(conn, seed, limit)?;
        "cooccurrence"
      } else {
        "item_embedding"
      };
      let suggestions = to_new(
        rows,
        profile_id,
        "because_you_watched",
        algorithm,
        Some(seed),
        ttl,
      );
      replace_recommendations(conn, profile_id, "because_you_watched", &suggestions)?;
    }

    // "Friends": titles watched by this user's friends.
    if let Some(user_id) = profile_owner(conn, profile_id)? {
      let rows = friends_watched(conn, user_id, profile_id, limit)?;
      let suggestions = to_new(rows, profile_id, "friends", "social", None, ttl);
      replace_recommendations(conn, profile_id, "friends", &suggestions)?;
    }
  }
  Ok(())
}

#[tokio::main]
async fn main() -> Result<(), WorkerError> {
  tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();

  let args = Args::parse();
  let locale = std::env::var("RECOMMENDATION_LOCALE").unwrap_or_else(|_| "en".to_string());
  let ttl_seconds: i64 = std::env::var("RECOMMENDER_TTL_SECS")
    .ok()
    .and_then(|value| value.parse().ok())
    .unwrap_or(86_400);
  let interval_seconds: u64 = std::env::var("RECOMMENDER_INTERVAL_SECS")
    .ok()
    .and_then(|value| value.parse().ok())
    .unwrap_or(3_600);

  let mut conn = dionysus_db::establish_connection();
  dionysus_db::run_migrations(&mut conn)?;
  let embedder = HttpEmbedder::from_env()?;

  let ttl = ChronoDuration::seconds(ttl_seconds);
  loop {
    let started = Instant::now();
    let embedded = backfill_embeddings(&mut conn, &embedder, args.batch_size).await?;
    refresh_caches(&mut conn, &embedder, &locale, args.limit, ttl)?;
    tracing::info!(
      embedded,
      elapsed_ms = started.elapsed().as_millis(),
      "recommendation pass complete"
    );
    if args.once {
      break;
    }
    tokio::time::sleep(std::time::Duration::from_secs(interval_seconds)).await;
  }

  Ok(())
}
