//! Metadata enrichment after a successful import: resolve ids, fetch from
//! the provider registry (TMDB primary, OMDb secondary, WASM plugins last —
//! see `dionysus_metadata_providers`), persist `media_translations` /
//! `media_external_ids` rows plus the authoritative release year, and
//! download images via [`ImageFetcher`].
//!
//! Enrichment never fails an import: the split files and `media` rows are
//! already committed, so every problem — fetch, DB write, image download —
//! comes back as a warning for the scan summary.

use std::path::PathBuf;

use diesel::prelude::*;
use uuid::Uuid;

use dionysus_db::models::{NewMediaExternalId, NewMediaTranslation, NewMetadataSource};
use dionysus_db::schema;
use dionysus_metadata::{
  model::{Descriptor, MediaKind, MediaMetadata, MediaQuery},
  ImageFetcher,
};
use dionysus_metadata_providers::MetadataFetcher;

use crate::parser::ParsedName;

/// `varchar(n)` counts characters, not bytes; providers can return longer
/// text (TMDB overviews run past 500), so clamp on the way in.
const MAX_TITLE: usize = 256;
const MAX_LOCALE: usize = 35;
const MAX_SYNOPSIS: usize = 500;
const MAX_SUMMARY: usize = 4000;
const MAX_SOURCE_ID: usize = 64;
const MAX_SOURCE_NAME: usize = 255;
const MAX_VERSION: usize = 64;

pub struct MetadataStep {
  fetcher: MetadataFetcher,
  images: ImageFetcher,
}

impl MetadataStep {
  /// Providers from `TMDB_API_KEY` / `OMDB_API_KEY`, plus every WASM
  /// plugin in `PLUGINS_DIR` (a low-priority provider each). With neither
  /// keys nor plugins this step is a no-op and imports run without
  /// metadata; a broken or duplicate plugin degrades to a warning.
  pub fn from_env(media_root: impl Into<PathBuf>) -> Self {
    let mut fetcher = MetadataFetcher::from_env();
    match dionysus_metadata_wasm::load_plugins_from_env() {
      Ok(plugins) => {
        for plugin in plugins {
          if let Err(err) = fetcher.register_plugin(std::sync::Arc::new(plugin)) {
            tracing::warn!("plugin skipped: {err}");
          }
        }
      }
      Err(err) => tracing::warn!("wasm plugins not loaded: {err}"),
    }
    Self {
      fetcher,
      images: ImageFetcher::with_default_domains(media_root),
    }
  }

  /// Fetch and store everything the providers know about this file. The
  /// returned warnings are non-fatal; a hard error (no provider matched, DB
  /// write failed) is the caller's to downgrade.
  pub async fn enrich(
    &self,
    conn: &mut PgConnection,
    media_id: Uuid,
    parsed: &ParsedName,
  ) -> Result<Vec<String>, String> {
    if !self.fetcher.enabled() {
      return Ok(Vec::new());
    }

    let query = media_query(parsed);
    let (mut warnings, merged, sources) = self.fetcher.fetch(&query).await?;
    warnings.extend(
      write_metadata_rows(conn, media_id, &merged, &sources)
        .map_err(|e| format!("db: {e}"))?,
    );

    if !merged.images.is_empty() {
      let (_, errors) = self.images.fetch_and_store(conn, media_id, &merged.images).await;
      warnings.extend(errors.into_iter().map(|e| format!("image: {e}")));
    }
    Ok(warnings)
  }
}

/// The parser's best-effort name → the provider lookup vocabulary.
fn media_query(parsed: &ParsedName) -> MediaQuery {
  MediaQuery {
    title: parsed.title.clone(),
    year: parsed.year,
    kind: if parsed.is_episode() {
      MediaKind::Episode
    } else {
      MediaKind::Movie
    },
    season: parsed.season,
    episode: parsed.episode,
    imdb_id: parsed.imdb_id.clone(),
    tmdb_id: None,
  }
}

/// `media_translations` + `media_external_ids` (+ the authoritative release
/// year) in one transaction — a media row is never half-enriched. Sources
/// that answered are upserted into the `metadata_source` registry. Returns
/// per-row warnings (`pub(crate)` for the DB round-trip test).
pub(crate) fn write_metadata_rows(
  conn: &mut PgConnection,
  media_id: Uuid,
  metadata: &MediaMetadata,
  sources: &[Descriptor],
) -> QueryResult<Vec<String>> {
  conn.transaction::<Vec<String>, diesel::result::Error, _>(|conn| {
    let mut warnings = Vec::new();

    // The filename parser's year is best-effort; the provider's wins.
    if let Some(year) = metadata.release_year {
      diesel::update(schema::media::table.find(media_id))
        .set(schema::media::release_year.eq(year))
        .execute(conn)?;
    }

    // One row per source that contributed; an existing row refreshes in
    // place, since the source may have been upgraded since the last import.
    for source in sources {
      diesel::insert_into(schema::metadata_source::table)
        .values(NewMetadataSource {
          id: clamp(&source.id, MAX_SOURCE_ID),
          name: clamp(&source.name, MAX_SOURCE_NAME),
          version: clamp(&source.version, MAX_VERSION),
          model_version: clamp(&source.model_version, MAX_VERSION),
        })
        .on_conflict(schema::metadata_source::id)
        .do_update()
        .set((
          schema::metadata_source::name.eq(clamp(&source.name, MAX_SOURCE_NAME)),
          schema::metadata_source::version.eq(clamp(&source.version, MAX_VERSION)),
          schema::metadata_source::model_version.eq(clamp(&source.model_version, MAX_VERSION)),
        ))
        .execute(conn)?;
    }

    for id in &metadata.external_ids {
      if id.value.trim().is_empty() {
        continue;
      }
      // UNIQUE(provider, value): one provider id maps to one media row, so
      // a second copy of the same title (different encode, re-added file)
      // keeps the first claim instead of erroring.
      let claimed = diesel::insert_into(schema::media_external_ids::table)
        .values(NewMediaExternalId {
          media_id,
          provider: &id.provider,
          value: &id.value,
        })
        .on_conflict((
          schema::media_external_ids::provider,
          schema::media_external_ids::value,
        ))
        .do_nothing()
        .execute(conn)?;
      if claimed == 0 {
        warnings.push(format!(
          "external id {}={} already belongs to another media",
          id.provider, id.value
        ));
      }
    }

    for text in &metadata.localized_texts {
      if text.title.is_empty() && text.synopsis.is_empty() && text.summary.is_empty() {
        continue;
      }
      let locale = if text.locale.trim().is_empty() { "und" } else { text.locale.as_str() };
      diesel::insert_into(schema::media_translations::table)
        .values(NewMediaTranslation {
          media_id,
          locale: clamp(locale, MAX_LOCALE),
          title: clamp(&text.title, MAX_TITLE),
          synopsis: clamp(&text.synopsis, MAX_SYNOPSIS),
          summary: clamp(&text.summary, MAX_SUMMARY),
          embedding: None,
        })
        .execute(conn)?;
    }
    Ok(warnings)
  })
}

/// Cut at `max` characters, keeping a valid UTF-8 boundary.
fn clamp(value: &str, max: usize) -> &str {
  match value.char_indices().nth(max) {
    Some((index, _)) => &value[..index],
    None => value,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use dionysus_db::models::{Media, NewMedia};
  use dionysus_db::types::media_kind::MediaKind as DbMediaKind;
  use dionysus_metadata::model::{Credit, ExternalId, LocalizedText, RemoteImage};

  #[test]
  fn clamp_cuts_on_characters_not_bytes() {
    // "é" is two bytes; a byte-based cut would panic or split the char.
    let long = "é".repeat(600);
    assert_eq!(clamp(&long, 500).chars().count(), 500);
    assert_eq!(clamp("short", 500), "short");
    assert_eq!(clamp("", 500), "");
    assert_eq!(clamp(&"é".repeat(500), 500).chars().count(), 500);
  }

  #[test]
  fn media_query_follows_the_filename() {
    let parsed = ParsedName {
      title: "Breaking Bad".to_string(),
      year: Some(2008),
      season: Some(3),
      episode: Some(7),
      imdb_id: Some("tt0903747".to_string()),
    };
    let query = media_query(&parsed);
    assert_eq!(query.kind, MediaKind::Episode);
    assert_eq!(query.season, Some(3));
    assert_eq!(query.imdb_id.as_deref(), Some("tt0903747"));

    let parsed = ParsedName {
      title: "The Matrix".to_string(),
      year: Some(1999),
      ..Default::default()
    };
    assert_eq!(media_query(&parsed).kind, MediaKind::Movie);
  }

  /// Requires a migrated database (CI provides one); skipped locally when
  /// DATABASE_URL is unset.
  #[test]
  fn write_metadata_rows_roundtrip() {
    if std::env::var("DATABASE_URL").is_err() {
      eprintln!("skipping write_metadata_rows_roundtrip: DATABASE_URL is not set");
      return;
    }
    let mut conn = dionysus_db::establish_connection();
    dionysus_db::run_migrations(&mut conn).expect("migrations must run");

    let media_id = Uuid::new_v4();
    diesel::insert_into(schema::media::table)
      .values(NewMedia {
        id: media_id,
        manifest_path: "metadata-test/video-0.mkv",
        kind: DbMediaKind::Movie,
        source_hash: Some(&format!("metadata-test-{media_id}")),
        duration_ms: None,
        release_year: None, // filled by the provider below
        season_number: None,
        episode_number: None,
        original_path: Some("/imports/The.Matrix.1999.mkv"),
      })
      .execute(&mut conn)
      .expect("insert media");

    // Multibyte synopsis over the 500-char column: clamped, not rejected.
    let metadata = MediaMetadata {
      release_year: Some(1999),
      kind: MediaKind::Movie,
      runtime_secs: Some(136 * 60),
      localized_texts: vec![
        LocalizedText {
          locale: "en".to_string(),
          title: "The Matrix".to_string(),
          synopsis: "é".repeat(600),
          summary: String::new(),
        },
        // All-empty entries never become rows.
        LocalizedText {
          locale: "fr".to_string(),
          title: String::new(),
          synopsis: String::new(),
          summary: String::new(),
        },
      ],
      images: vec![RemoteImage {
        url: "https://image.tmdb.org/t/p/original/poster.jpg".to_string(),
        kind: dionysus_metadata::model::MediaImageKind::Poster,
        locale: None,
      }],
      credits: vec![Credit {
        person_external_ids: vec![ExternalId {
          provider: "tmdb".to_string(),
          value: "6384".to_string(),
        }],
        name: "Keanu Reeves".to_string(),
        roles: vec![dionysus_metadata::model::CreditRole::Actor],
        characters: vec!["Neo".to_string()],
        billing: 0,
      }],
      external_ids: vec![
        ExternalId { provider: "tmdb".to_string(), value: "603".to_string() },
        ExternalId { provider: "imdb".to_string(), value: "tt0133093".to_string() },
      ],
    };

    let sources = vec![
      Descriptor::new("tmdb", "TMDB", "0.1.0"),
      Descriptor::new("omdb", "OMDb", "0.1.0"),
    ];
    let warnings = write_metadata_rows(&mut conn, media_id, &metadata, &sources).expect("rows");
    assert!(warnings.is_empty(), "{warnings:?}");

    let stored: Media = schema::media::table.find(media_id).first(&mut conn).expect("media");
    assert_eq!(stored.release_year, Some(1999), "provider year wins");

    let translations = dionysus_db::models::MediaTranslation::belonging_to(&stored)
      .load::<dionysus_db::models::MediaTranslation>(&mut conn)
      .expect("translations");
    assert_eq!(translations.len(), 1, "empty locales never become rows");
    assert_eq!(translations[0].locale, "en");
    assert_eq!(translations[0].title, "The Matrix");
    assert_eq!(translations[0].synopsis.chars().count(), MAX_SYNOPSIS);

    let ids = dionysus_db::models::MediaExternalId::belonging_to(&stored)
      .load::<dionysus_db::models::MediaExternalId>(&mut conn)
      .expect("external ids");
    assert_eq!(ids.len(), 2);

    // Both answering sources landed in the registry with their descriptors.
    let stored_sources: Vec<dionysus_db::models::MetadataSource> = schema::metadata_source::table
      .filter(schema::metadata_source::id.eq_any(["tmdb", "omdb"]))
      .load(&mut conn)
      .expect("sources");
    assert_eq!(stored_sources.len(), 2);
    let omdb = stored_sources.iter().find(|s| s.id == "omdb").expect("omdb row");
    assert_eq!(omdb.name, "OMDb");
    assert_eq!(omdb.version, "0.1.0");
    assert_eq!(omdb.model_version, dionysus_metadata::model::MODEL_VERSION);

    // A second copy of the same title: the UNIQUE(provider, value) claim
    // stays with the first media, reported as a warning.
    let twin_id = Uuid::new_v4();
    diesel::insert_into(schema::media::table)
      .values(NewMedia {
        id: twin_id,
        manifest_path: "metadata-test/twin/video-0.mkv",
        kind: DbMediaKind::Movie,
        source_hash: Some(&format!("metadata-test-twin-{twin_id}")),
        duration_ms: None,
        release_year: None,
        season_number: None,
        episode_number: None,
        original_path: Some("/imports/The.Matrix.1080p.mkv"),
      })
      .execute(&mut conn)
      .expect("insert twin");
    let warnings = write_metadata_rows(&mut conn, twin_id, &metadata, &sources).expect("twin rows");
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(warnings[0].contains("tmdb=603"), "{warnings:?}");
    assert!(warnings[1].contains("imdb=tt0133093"), "{warnings:?}");

    // Registry rows are upserted by id, never duplicated.
    let count: i64 = schema::metadata_source::table
      .filter(schema::metadata_source::id.eq_any(["tmdb", "omdb"]))
      .count()
      .get_result(&mut conn)
      .expect("source count");
    assert_eq!(count, 2);

    // Cleanup (cascade-checks the FKs while we're at it).
    diesel::delete(schema::media::table.find(media_id))
      .execute(&mut conn)
      .expect("cleanup");
    diesel::delete(schema::media::table.find(twin_id))
      .execute(&mut conn)
      .expect("cleanup twin");
    diesel::delete(
      schema::metadata_source::table.filter(schema::metadata_source::id.eq_any(["tmdb", "omdb"])),
    )
    .execute(&mut conn)
    .expect("cleanup sources");
  }
}
