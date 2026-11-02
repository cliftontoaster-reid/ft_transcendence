//! Resolution (plan §Phase 5): turn a parsed filename query into concrete
//! provider ids before anything is fetched.
//!
//! - Parser found a `tt…` id → OMDb can fetch it directly, and TMDB is
//!   cross-linked through `/find`.
//! - Otherwise → search TMDB by title+year and take the best-scoring hit.
//! - The OMDb-by-title last resort (no TMDB key, no IMDb id in the name)
//!   lives in the importer's fetch step, since it needs the registry.

use tracing::warn;

use dionysus_metadata::MetadataProvider;
use dionysus_metadata_model::{MediaKind, MediaQuery};

use crate::TmdbProvider;

/// Provider ids resolved for one query. Ids may be `None` — callers fetch
/// whatever exists and treat the rest as "this source has nothing".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedIds {
  /// TMDB id, prefixed `movie:<id>` / `tv:<id>`.
  pub tmdb_id: Option<String>,
  /// IMDb id, from the filename parser (cross-linking from the fetched TMDB
  /// record happens later, once there is a record to read).
  pub imdb_id: Option<String>,
  /// Resolution failures (surfaced to the caller as warnings — a failed
  /// cross-link means one provider silently sits out this media).
  pub warnings: Vec<String>,
}

/// Resolve a query to provider ids. Errors from TMDB (search or find) are
/// logged, recorded as warnings, and degrade to `None` — resolution is
/// best-effort, and the importer still tries whatever remains.
pub async fn resolve_ids(query: &MediaQuery, tmdb: Option<&TmdbProvider>) -> ResolvedIds {
  let mut resolved = ResolvedIds {
    tmdb_id: query.tmdb_id.as_deref().map(|id| encode_tmdb_id(id, query.kind)),
    imdb_id: query.imdb_id.clone(),
    ..Default::default()
  };
  if resolved.tmdb_id.is_some() || tmdb.is_none() {
    return resolved;
  }

  let outcome = match &resolved.imdb_id {
    Some(imdb) => tmdb
      .expect("checked above")
      .find_by_imdb(imdb, query.kind)
      .await,
    None => tmdb
      .expect("checked above")
      .search(query)
      .await
      .map(|results| results.into_iter().next().map(|r| r.id)),
  };
  match outcome {
    Ok(id) => resolved.tmdb_id = id,
    Err(err) => {
      warn!(provider = "tmdb", %err, "id resolution failed");
      resolved.warnings.push(format!("tmdb: {err}"));
    }
  }
  resolved
}

/// A bare TMDB id is ambiguous between movie and tv; tag it with the
/// query's kind. Already-prefixed ids pass through untouched.
fn encode_tmdb_id(raw: &str, kind: MediaKind) -> String {
  if raw.contains(':') {
    raw.to_string()
  } else {
    let segment = match kind {
      MediaKind::Movie => "movie",
      MediaKind::Episode => "tv",
    };
    format!("{segment}:{raw}")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::MockServer;

  fn query() -> MediaQuery {
    MediaQuery {
      title: "The Matrix".to_string(),
      year: Some(1999),
      imdb_id: Some("tt0133093".to_string()),
      ..Default::default()
    }
  }

  #[tokio::test]
  async fn imdb_id_goes_straight_to_find_and_skips_search() {
    let server = MockServer::serve_json(r#"{"movie_results":[{"id":603}],"tv_results":[]}"#.to_string())
      .await;
    let tmdb = TmdbProvider::new("k").with_base_url(server.base_url());

    let resolved = resolve_ids(&query(), Some(&tmdb)).await;
    assert_eq!(resolved.tmdb_id.as_deref(), Some("movie:603"));
    assert_eq!(resolved.imdb_id.as_deref(), Some("tt0133093"));
    assert_eq!(server.request_count(), 1, "find must not trigger a search");
    assert!(server.request(0).contains("/find/tt0133093"));
  }

  #[tokio::test]
  async fn without_an_imdb_id_the_title_search_picks_the_best_hit() {
    let server = MockServer::serve_json(
      r#"{"results":[
        {"id":604,"title":"The Matrix Reloaded","release_date":"2003-05-15"},
        {"id":603,"title":"The Matrix","release_date":"1999-03-30"}
      ]}"#
      .to_string(),
    )
    .await;
    let tmdb = TmdbProvider::new("k").with_base_url(server.base_url());

    let no_imdb = MediaQuery { imdb_id: None, ..query() };
    let resolved = resolve_ids(&no_imdb, Some(&tmdb)).await;
    assert_eq!(resolved.tmdb_id.as_deref(), Some("movie:603"));
    assert!(server.request(0).contains("/search/movie"));
  }

  #[tokio::test]
  async fn no_provider_yields_no_ids() {
    let resolved = resolve_ids(&query(), None).await;
    assert_eq!(resolved.tmdb_id, None);
    // The parser's IMDb id survives even without TMDB — OMDb can use it.
    assert_eq!(resolved.imdb_id.as_deref(), Some("tt0133093"));
  }

  #[tokio::test]
  async fn bare_tmdb_ids_get_kind_prefixed() {
    let query = MediaQuery {
      tmdb_id: Some("603".to_string()),
      ..Default::default()
    };
    let resolved = resolve_ids(&query, None).await;
    assert_eq!(resolved.tmdb_id.as_deref(), Some("movie:603"));

    let query = MediaQuery {
      tmdb_id: Some("1396".to_string()),
      kind: MediaKind::Episode,
      ..Default::default()
    };
    let resolved = resolve_ids(&query, None).await;
    assert_eq!(resolved.tmdb_id.as_deref(), Some("tv:1396"));
  }

  #[tokio::test]
  async fn tmdb_failure_degrades_instead_of_erroring() {
    let server = MockServer::serve_status(500, "boom").await;
    let tmdb = TmdbProvider::new("k").with_base_url(server.base_url());

    // find fails → no TMDB id, but the parser's IMDb id survives for OMDb.
    let resolved = resolve_ids(&query(), Some(&tmdb)).await;
    assert_eq!(resolved.tmdb_id, None);
    assert_eq!(resolved.imdb_id.as_deref(), Some("tt0133093"));

    // search fails the same way.
    let no_imdb = MediaQuery { imdb_id: None, ..query() };
    let resolved = resolve_ids(&no_imdb, Some(&tmdb)).await;
    assert_eq!(resolved.tmdb_id, None);
    assert_eq!(resolved.imdb_id, None);
  }
}
