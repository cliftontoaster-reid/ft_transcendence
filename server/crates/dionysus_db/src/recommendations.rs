//! Queries backing the recommendation service and worker.
//!
//! The ranking queries use raw SQL because they rely on Postgres features
//! (window-free co-occurrence grouping, pgvector's `<=>` operator) that don't
//! map cleanly onto Diesel's typed DSL. Result rows are still decoded through
//! `QueryableByName`, and all parameters are bound rather than interpolated.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use pgvector::{Vector as PgVector, sql_types::Vector as VectorSql};
use uuid::Uuid;

use crate::{
  models::{NewMediaRating, NewRecommendation, NewRecommendationEvent, Recommendation},
  schema,
};

/// A title plus its ranking score, shaped for the proto `Recommendation`.
#[derive(QueryableByName, Debug, Clone)]
pub struct ScoredMedia {
  #[diesel(sql_type = diesel::sql_types::Uuid)]
  pub media_id: Uuid,
  #[diesel(sql_type = diesel::sql_types::Varchar)]
  pub title: String,
  #[diesel(sql_type = diesel::sql_types::Float4)]
  pub score: f32,
}

/// Embeddings of titles a profile has interacted with; used to build the
/// profile centroid in Rust (pgvector has no portable `avg(vector)`).
#[derive(QueryableByName, Debug)]
pub struct EmbeddingRow {
  #[diesel(sql_type = diesel::sql_types::Uuid)]
  pub media_id: Uuid,
  #[diesel(sql_type = VectorSql)]
  pub embedding: PgVector,
}

/// A translation whose embedding still needs generating.
#[derive(QueryableByName, Debug)]
pub struct PendingTranslation {
  #[diesel(sql_type = diesel::sql_types::Uuid)]
  pub id: Uuid,
  #[diesel(sql_type = diesel::sql_types::Varchar)]
  pub title: String,
  #[diesel(sql_type = diesel::sql_types::Varchar)]
  pub synopsis: String,
  #[diesel(sql_type = diesel::sql_types::Varchar)]
  pub summary: String,
}

impl PendingTranslation {
  /// The text fed to the embedding backend. Title first so it dominates the
  /// resulting vector.
  pub fn embed_text(&self) -> String {
    [
      self.title.as_str(),
      self.synopsis.as_str(),
      self.summary.as_str(),
    ]
    .into_iter()
    .filter(|part| !part.trim().is_empty())
    .collect::<Vec<_>>()
    .join("\n")
  }
}

/// Recency-weighted popularity across all profiles.
pub fn trending(
  conn: &mut PgConnection,
  since: DateTime<Utc>,
  limit: i64,
) -> QueryResult<Vec<ScoredMedia>> {
  diesel::sql_query(
    r#"
    SELECT h.media_id AS media_id,
           COALESCE((
             SELECT t.title FROM media_translations t
             WHERE t.media_id = h.media_id
             ORDER BY (t.locale = 'en') DESC, t.locale
             LIMIT 1
           ), '') AS title,
           count(*)::real AS score
    FROM watch_history h
    WHERE h.watched_at >= $1
    GROUP BY h.media_id
    ORDER BY score DESC, h.media_id
    LIMIT $2
    "#,
  )
  .bind::<diesel::sql_types::Timestamptz, _>(since)
  .bind::<diesel::sql_types::Int8, _>(limit)
  .load(conn)
}

/// Item-to-item co-occurrence for a single seed title.
pub fn similar_by_cooccurrence(
  conn: &mut PgConnection,
  media_id: Uuid,
  limit: i64,
) -> QueryResult<Vec<ScoredMedia>> {
  diesel::sql_query(
    r#"
    WITH seed AS (SELECT $1::uuid AS media_id)
    SELECT b.media_id AS media_id,
           COALESCE((
             SELECT t.title FROM media_translations t
             WHERE t.media_id = b.media_id
             ORDER BY (t.locale = 'en') DESC, t.locale
             LIMIT 1
           ), '') AS title,
           count(*)::real AS score
    FROM watch_history a
    JOIN watch_history b
      ON b.profile_id = a.profile_id AND b.media_id <> a.media_id
    WHERE a.media_id IN (SELECT media_id FROM seed)
    GROUP BY b.media_id
    ORDER BY score DESC, b.media_id
    LIMIT $2
    "#,
  )
  .bind::<diesel::sql_types::Uuid, _>(media_id)
  .bind::<diesel::sql_types::Int8, _>(limit)
  .load(conn)
}

/// Items watched alongside this profile's seeds, excluding everything the
/// profile has already seen or saved.
pub fn for_profile_by_cooccurrence(
  conn: &mut PgConnection,
  profile_id: Uuid,
  limit: i64,
) -> QueryResult<Vec<ScoredMedia>> {
  diesel::sql_query(
    r#"
    WITH seed AS (
      SELECT media_id FROM watch_history WHERE profile_id = $1
      UNION
      SELECT media_id FROM my_list WHERE profile_id = $1
    ),
    seen AS (
      SELECT media_id FROM watch_history WHERE profile_id = $1
      UNION
      SELECT media_id FROM my_list WHERE profile_id = $1
    )
    SELECT b.media_id AS media_id,
           COALESCE((
             SELECT t.title FROM media_translations t
             WHERE t.media_id = b.media_id
             ORDER BY (t.locale = 'en') DESC, t.locale
             LIMIT 1
           ), '') AS title,
           count(*)::real AS score
    FROM watch_history a
    JOIN watch_history b
      ON b.profile_id = a.profile_id AND b.media_id <> a.media_id
    WHERE a.media_id IN (SELECT media_id FROM seed)
      AND b.media_id NOT IN (SELECT media_id FROM seen)
    GROUP BY b.media_id
    ORDER BY score DESC, b.media_id
    LIMIT $2
    "#,
  )
  .bind::<diesel::sql_types::Uuid, _>(profile_id)
  .bind::<diesel::sql_types::Int8, _>(limit)
  .load(conn)
}

/// Nearest neighbours of a seed title's embedding, in the same locale.
pub fn similar_by_embedding(
  conn: &mut PgConnection,
  media_id: Uuid,
  locale: &str,
  limit: i64,
) -> QueryResult<Vec<ScoredMedia>> {
  diesel::sql_query(
    r#"
    SELECT t.media_id AS media_id,
           t.title AS title,
           (1 - (t.embedding <=> seed.embedding))::real AS score
    FROM media_translations t
    JOIN media_translations seed
      ON seed.media_id = $1 AND seed.locale = $2 AND seed.embedding IS NOT NULL
    WHERE t.locale = $2
      AND t.embedding IS NOT NULL
      AND t.media_id <> $1
    ORDER BY t.embedding <=> seed.embedding
    LIMIT $3
    "#,
  )
  .bind::<diesel::sql_types::Uuid, _>(media_id)
  .bind::<diesel::sql_types::Varchar, _>(locale)
  .bind::<diesel::sql_types::Int8, _>(limit)
  .load(conn)
}

/// Titles seen by the profile's friends but not by the profile itself.
pub fn friends_watched(
  conn: &mut PgConnection,
  user_id: Uuid,
  profile_id: Uuid,
  limit: i64,
) -> QueryResult<Vec<ScoredMedia>> {
  diesel::sql_query(
    r#"
    SELECT h.media_id AS media_id,
           COALESCE((
             SELECT t.title FROM media_translations t
             WHERE t.media_id = h.media_id
             ORDER BY (t.locale = 'en') DESC, t.locale
             LIMIT 1
           ), '') AS title,
           count(*)::real AS score
    FROM user_friends f
    JOIN viewer_profiles vp ON vp.user_id = f.friend_id
    JOIN watch_history h ON h.profile_id = vp.id
    WHERE f.user_id = $1
      AND h.media_id NOT IN (
        SELECT media_id FROM watch_history WHERE profile_id = $2
        UNION
        SELECT media_id FROM my_list WHERE profile_id = $2
      )
    GROUP BY h.media_id
    ORDER BY score DESC, h.media_id
    LIMIT $3
    "#,
  )
  .bind::<diesel::sql_types::Uuid, _>(user_id)
  .bind::<diesel::sql_types::Uuid, _>(profile_id)
  .bind::<diesel::sql_types::Int8, _>(limit)
  .load(conn)
}

pub fn profile_owner(conn: &mut PgConnection, profile_id: Uuid) -> QueryResult<Option<Uuid>> {
  schema::viewer_profiles::table
    .find(profile_id)
    .select(schema::viewer_profiles::user_id)
    .first(conn)
    .optional()
}

pub fn profile_id_for_user(conn: &mut PgConnection, user_id: Uuid) -> QueryResult<Option<Uuid>> {
  schema::viewer_profiles::table
    .filter(schema::viewer_profiles::user_id.eq(user_id))
    .select(schema::viewer_profiles::id)
    .first(conn)
    .optional()
}

pub fn list_profile_ids(conn: &mut PgConnection) -> QueryResult<Vec<Uuid>> {
  schema::viewer_profiles::table
    .select(schema::viewer_profiles::id)
    .load(conn)
}

/// Most recently watched title for a profile, used as the seed of a
/// "because you watched" row.
pub fn latest_watched_media(
  conn: &mut PgConnection,
  profile_id: Uuid,
) -> QueryResult<Option<Uuid>> {
  use schema::watch_history::dsl as h;

  h::watch_history
    .filter(h::profile_id.eq(profile_id))
    .order(h::watched_at.desc())
    .select(h::media_id)
    .first(conn)
    .optional()
}

/// Embeddings for the titles a profile watched or saved.
pub fn positive_embeddings(
  conn: &mut PgConnection,
  profile_id: Uuid,
  locale: &str,
) -> QueryResult<Vec<EmbeddingRow>> {
  diesel::sql_query(
    r#"
    SELECT DISTINCT t.media_id AS media_id, t.embedding AS embedding
    FROM media_translations t
    WHERE t.locale = $1
      AND t.embedding IS NOT NULL
      AND (
        t.media_id IN (SELECT media_id FROM watch_history WHERE profile_id = $2)
        OR t.media_id IN (SELECT media_id FROM my_list WHERE profile_id = $2)
      )
    "#,
  )
  .bind::<diesel::sql_types::Varchar, _>(locale)
  .bind::<diesel::sql_types::Uuid, _>(profile_id)
  .load(conn)
}

/// Nearest neighbours of an arbitrary vector, excluding seen/saved items.
pub fn nearest_to_embedding(
  conn: &mut PgConnection,
  embedding: &PgVector,
  locale: &str,
  profile_id: Uuid,
  limit: i64,
) -> QueryResult<Vec<ScoredMedia>> {
  diesel::sql_query(
    r#"
    SELECT t.media_id AS media_id,
           t.title AS title,
           (1 - (t.embedding <=> $1))::real AS score
    FROM media_translations t
    WHERE t.locale = $2
      AND t.embedding IS NOT NULL
      AND t.media_id NOT IN (
        SELECT media_id FROM watch_history WHERE profile_id = $3
        UNION
        SELECT media_id FROM my_list WHERE profile_id = $3
      )
    ORDER BY t.embedding <=> $1
    LIMIT $4
    "#,
  )
  .bind::<VectorSql, _>(embedding.clone())
  .bind::<diesel::sql_types::Varchar, _>(locale)
  .bind::<diesel::sql_types::Uuid, _>(profile_id)
  .bind::<diesel::sql_types::Int8, _>(limit)
  .load(conn)
}

pub fn pending_translations(
  conn: &mut PgConnection,
  limit: i64,
) -> QueryResult<Vec<PendingTranslation>> {
  diesel::sql_query(
    r#"
    SELECT id, title, synopsis, summary
    FROM media_translations
    WHERE embedding IS NULL
    ORDER BY id
    LIMIT $1
    "#,
  )
  .bind::<diesel::sql_types::Int8, _>(limit)
  .load(conn)
}

pub fn set_translation_embedding(
  conn: &mut PgConnection,
  translation_id: Uuid,
  embedding: PgVector,
) -> QueryResult<()> {
  diesel::update(schema::media_translations::table.find(translation_id))
    .set(schema::media_translations::embedding.eq(Some(embedding)))
    .execute(conn)?;
  Ok(())
}

/// Replace one reason's cached rows atomically so readers never see a
/// half-written list.
pub fn replace_recommendations(
  conn: &mut PgConnection,
  profile_id: Uuid,
  reason: &str,
  rows: &[NewRecommendation<'_>],
) -> QueryResult<()> {
  use schema::recommendation::dsl as r;

  conn.transaction::<(), diesel::result::Error, _>(|conn| {
    diesel::delete(
      r::recommendation
        .filter(r::profile_id.eq(profile_id))
        .filter(r::reason.eq(reason)),
    )
    .execute(conn)?;
    for row in rows {
      diesel::insert_into(r::recommendation)
        .values(row)
        .execute(conn)?;
    }
    Ok(())
  })
}

pub fn list_recommendations(
  conn: &mut PgConnection,
  profile_id: Uuid,
  reason: &str,
  limit: i64,
) -> QueryResult<Vec<Recommendation>> {
  use schema::recommendation::dsl as r;

  r::recommendation
    .filter(r::profile_id.eq(profile_id))
    .filter(r::reason.eq(reason))
    .order(r::score.desc())
    .limit(limit)
    .select(Recommendation::as_select())
    .load(conn)
}

/// Cached rows joined to a display title, in the same shape as the live
/// ranking queries so the service can use one conversion path.
pub fn cached_suggestions(
  conn: &mut PgConnection,
  profile_id: Uuid,
  reason: &str,
  limit: i64,
) -> QueryResult<Vec<ScoredMedia>> {
  diesel::sql_query(
    r#"
    SELECT rec.media_id AS media_id,
           COALESCE((
             SELECT t.title FROM media_translations t
             WHERE t.media_id = rec.media_id
             ORDER BY (t.locale = 'en') DESC, t.locale
             LIMIT 1
           ), '') AS title,
           rec.score AS score
    FROM recommendation rec
    WHERE rec.profile_id = $1 AND rec.reason = $2
    ORDER BY rec.score DESC
    LIMIT $3
    "#,
  )
  .bind::<diesel::sql_types::Uuid, _>(profile_id)
  .bind::<diesel::sql_types::Varchar, _>(reason)
  .bind::<diesel::sql_types::Int8, _>(limit)
  .load(conn)
}

pub fn upsert_rating(
  conn: &mut PgConnection,
  profile_id: Uuid,
  media_id: Uuid,
  value: i16,
) -> QueryResult<()> {
  use schema::media_rating::dsl as mr;

  diesel::insert_into(mr::media_rating)
    .values(NewMediaRating {
      profile_id,
      media_id,
      value,
    })
    .on_conflict((mr::profile_id, mr::media_id))
    .do_update()
    .set(mr::value.eq(value))
    .execute(conn)?;
  Ok(())
}

pub fn record_event(
  conn: &mut PgConnection,
  profile_id: Uuid,
  media_id: Uuid,
  kind: &str,
) -> QueryResult<()> {
  diesel::insert_into(schema::recommendation_event::table)
    .values(NewRecommendationEvent {
      profile_id,
      media_id,
      kind,
    })
    .execute(conn)?;
  Ok(())
}
