//! Fetch orchestration: resolve a [`MediaQuery`] to provider ids, fetch in
//! priority order, cross-link IMDb↔TMDB, and merge. This is what the
//! importer runs after every successful split; WASM providers join the same
//! registry via [`MetadataFetcher::register_plugin`] and resolve after the
//! natives.
//!
//! Failure policy: a provider that errors becomes a *warning* and the rest
//! still contribute — only "nobody answered at all" is an error.

use std::sync::Arc;

use tracing::info;

use dionysus_metadata::{merge_metadata, MetadataProvider, ProviderRegistry};
use dionysus_metadata_model::{Descriptor, MediaMetadata, MediaQuery};

use crate::{resolve_ids, OmdbProvider, ResolvedIds, TmdbProvider};

/// TMDB is the primary source (full texts, credits, images), OMDb the
/// secondary backfill — merge order follows these, not registration luck.
const TMDB_PRIORITY: i32 = 10;
const OMDB_PRIORITY: i32 = 5;
/// WASM plugins refine below both natives: they can fill gaps, never
/// override a native's answer.
const PLUGIN_PRIORITY: i32 = 1;
/// The native provider ids — everything else in the registry is a plugin.
const NATIVE_IDS: &[&str] = &["tmdb", "omdb"];

/// The configured provider set: registry + concrete handles for the
/// id-resolution steps the trait alone can't express (TMDB `/find`,
/// OMDb title search).
pub struct MetadataFetcher {
  registry: ProviderRegistry,
  tmdb: Option<Arc<TmdbProvider>>,
  omdb: Option<Arc<OmdbProvider>>,
}

impl MetadataFetcher {
  /// Native providers from `TMDB_API_KEY` (v3 key or v4 token) and
  /// `OMDB_API_KEY`. With neither set, [`enabled`](Self::enabled) is false
  /// and the importer skips enrichment entirely.
  pub fn from_env() -> Self {
    let tmdb = TmdbProvider::from_env().map(Arc::new);
    let omdb = OmdbProvider::from_env().map(Arc::new);

    let mut registry = ProviderRegistry::new();
    if let Some(provider) = &tmdb {
      registry.register(provider.clone(), TMDB_PRIORITY);
    }
    if let Some(provider) = &omdb {
      registry.register(provider.clone(), OMDB_PRIORITY);
    }
    let ids = registry.ids();
    if ids.is_empty() {
      info!("metadata disabled: TMDB_API_KEY / OMDB_API_KEY are not set");
    } else {
      info!(providers = ?ids, "metadata providers configured");
    }
    Self { registry, tmdb, omdb }
  }

  pub fn enabled(&self) -> bool {
    !self.registry.ids().is_empty()
  }

  /// Register a WASM-backed (or any third-party) provider at
  /// [`PLUGIN_PRIORITY`]. The importer calls this for every plugin loaded
  /// from `PLUGINS_DIR`; duplicate ids are rejected so the plugin loop in
  /// [`Self::fetch`] stays unambiguous.
  pub fn register_plugin(
    &mut self,
    provider: Arc<dyn MetadataProvider>,
  ) -> Result<(), String> {
    let id = provider.id().to_string();
    if self.registry.ids().contains(&id.as_str()) {
      return Err(format!("provider id {id} already registered"));
    }
    info!(provider = %id, "registered plugin metadata provider");
    self.registry.register(provider, PLUGIN_PRIORITY);
    Ok(())
  }

  /// Resolve + fetch + merge for one query. Returns collected per-provider
  /// warnings alongside the merged metadata and the descriptors of every
  /// source that contributed; errors only when no provider produced
  /// anything.
  pub async fn fetch(
    &self,
    query: &MediaQuery,
  ) -> Result<(Vec<String>, MediaMetadata, Vec<Descriptor>), String> {
    if !self.enabled() {
      return Err("no metadata providers configured".to_string());
    }
    let mut warnings: Vec<String> = Vec::new();
    let mut sources: Vec<Descriptor> = Vec::new();
    let ResolvedIds {
      tmdb_id,
      mut imdb_id,
      warnings: resolution_warnings,
    } = resolve_ids(query, self.tmdb.as_deref()).await;
    warnings.extend(resolution_warnings);
    let mut parts: Vec<(i32, MediaMetadata)> = Vec::new();

    // 1. TMDB first — its record is also where a filename's bare title
    //    turns into an IMDb id.
    if let Some(id) = &tmdb_id {
      match self.registry.fetch_one("tmdb", id).await {
        Ok(metadata) => {
          if imdb_id.is_none() {
            imdb_id = metadata
              .external_ids
              .iter()
              .find(|e| e.provider == "imdb")
              .map(|e| e.value.clone());
          }
          parts.push((TMDB_PRIORITY, metadata));
          if let Some(descriptor) = self.registry.descriptor("tmdb") {
            sources.push(descriptor);
          }
        }
        Err(err) => warnings.push(format!("tmdb: {err}")),
      }
    }

    // 2. OMDb directly by IMDb id — from the filename parser or the
    //    cross-link above.
    if let Some(imdb) = &imdb_id
      && self.omdb.is_some()
    {
      match self.registry.fetch_one("omdb", imdb).await {
        Ok(metadata) => {
          parts.push((OMDB_PRIORITY, metadata));
          if let Some(descriptor) = self.registry.descriptor("omdb") {
            sources.push(descriptor);
          }
        }
        Err(err) => warnings.push(format!("omdb: {err}")),
      }
    }

    // 3. Last resort: nobody has been resolved yet (no TMDB key, no IMDb id
    //    in the file name, empty TMDB search) — search OMDb by title.
    if parts.is_empty()
      && let Some(omdb) = &self.omdb
    {
      match omdb.search(query).await {
        Ok(results) => match results.into_iter().next() {
          Some(best) => match omdb.fetch(&best.id).await {
            Ok(metadata) => {
              parts.push((OMDB_PRIORITY, metadata));
              if let Some(descriptor) = self.registry.descriptor("omdb") {
                sources.push(descriptor);
              }
            }
            Err(err) => warnings.push(format!("omdb: {err}")),
          },
          None => warnings.push("omdb: no search results".to_string()),
        },
        Err(err) => warnings.push(format!("omdb: {err}")),
      }
    }

    // 4. WASM plugins, lowest priority: ask each one directly so a plugin
    //    failure degrades to its own warning instead of failing the batch.
    //    Highest-scoring search hit wins; an empty search is a legitimate
    //    miss (movies on a TV source), so it stays silent.
    for plugin_id in self.registry.ids_except(NATIVE_IDS) {
      let hits = match self.registry.search_one(&plugin_id, query).await {
        Ok(hits) => hits,
        Err(err) => {
          warnings.push(format!("{plugin_id}: {err}"));
          continue;
        }
      };
      let Some(best) = hits.into_iter().next() else { continue };
      match self.registry.fetch_one(&plugin_id, &best.id).await {
        Ok(metadata) => {
          let priority = self.registry.priority(&plugin_id).unwrap_or(PLUGIN_PRIORITY);
          parts.push((priority, metadata));
          if let Some(descriptor) = self.registry.descriptor(&plugin_id) {
            sources.push(descriptor);
          }
        }
        Err(err) => warnings.push(format!("{plugin_id}: {err}")),
      }
    }

    if parts.is_empty() {
      let detail = if warnings.is_empty() {
        "no provider had a match for this media".to_string()
      } else {
        warnings.join("; ")
      };
      return Err(detail);
    }
    // Priority order: TMDB wins every field-level contest, plugins only
    // ever fill gaps. Stable sort keeps registration order on ties.
    parts.sort_by_key(|(priority, _)| std::cmp::Reverse(*priority));
    Ok((
      warnings,
      merge_metadata(parts.into_iter().map(|(_, m)| m).collect()),
      sources,
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::MockServer;

  fn matrix_query() -> MediaQuery {
    MediaQuery {
      title: "The Matrix".to_string(),
      year: Some(1999),
      ..Default::default()
    }
  }

  fn with_imdb(mut query: MediaQuery) -> MediaQuery {
    query.imdb_id = Some("tt0133093".to_string());
    query
  }

  const MOVIE_DETAILS: &str = r#"{
    "title": "The Matrix",
    "overview": "A hacker learns the truth about reality.",
    "release_date": "1999-03-30",
    "runtime": 136,
    "original_language": "en",
    "imdb_id": "tt0133093",
    "credits": {"cast": [], "crew": []},
    "translations": {"translations": [
      {"iso_639_1": "en", "data": {"title": "The Matrix", "overview": "A hacker learns the truth about reality."}}
    ]},
    "images": {"posters": [], "backdrops": []}
  }"#;

  const OMDB_TITLE: &str = r#"{
    "Title": "The Matrix", "Year": "1999", "Runtime": "136 min",
    "Plot": "Neo joins the resistance.", "Poster": "N/A",
    "Type": "movie", "imdbID": "tt0133093", "Response": "True"
  }"#;

  /// Fetcher wired to mock servers instead of the real APIs.
  fn fetcher(tmdb: Option<&MockServer>, omdb: Option<&MockServer>) -> MetadataFetcher {
    let tmdb = tmdb.map(|s| Arc::new(TmdbProvider::new("k").with_base_url(s.base_url())));
    let omdb = omdb.map(|s| Arc::new(OmdbProvider::new("k").with_base_url(s.base_url())));
    let mut registry = ProviderRegistry::new();
    if let Some(provider) = &tmdb {
      registry.register(provider.clone(), TMDB_PRIORITY);
    }
    if let Some(provider) = &omdb {
      registry.register(provider.clone(), OMDB_PRIORITY);
    }
    MetadataFetcher { registry, tmdb, omdb }
  }

  #[tokio::test]
  async fn cross_links_imdb_from_tmdb_and_merges_both_providers() {
    // TMDB serves /find first, then the movie details (one server, in order).
    let tmdb = MockServer::serve_sequence(vec![
      r#"{"movie_results":[{"id":603}],"tv_results":[]}"#.to_string(),
      MOVIE_DETAILS.to_string(),
    ])
    .await;
    let omdb = MockServer::serve_json(OMDB_TITLE.to_string()).await;
    let fetcher = fetcher(Some(&tmdb), Some(&omdb));

    let (warnings, merged, sources) = fetcher.fetch(&with_imdb(matrix_query())).await.expect("fetch");
    assert!(warnings.is_empty(), "{warnings:?}");

    // Both providers contributed: texts merged per field (TMDB title,
    // OMDb summary), external ids unioned.
    assert_eq!(merged.release_year, Some(1999));
    let en = merged
      .localized_texts
      .iter()
      .find(|t| t.locale == "en")
      .expect("en");
    assert_eq!(en.title, "The Matrix");
    assert_eq!(en.synopsis, "A hacker learns the truth about reality.");
    assert_eq!(en.summary, "Neo joins the resistance.");
    assert!(merged.external_ids.iter().any(|e| e.provider == "tmdb" && e.value == "603"));
    assert!(merged.external_ids.iter().any(|e| e.provider == "imdb" && e.value == "tt0133093"));

    let ids: Vec<&str> = sources.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["tmdb", "omdb"]);
    assert_eq!(sources[0].name, "TMDB");
    assert_eq!(sources[1].name, "OMDb");

    assert_eq!(tmdb.request_count(), 2, "find + details");
    assert!(tmdb.request(0).contains("/find/tt0133093"));
    assert!(tmdb.request(1).contains("/movie/603"));
    assert_eq!(omdb.request_count(), 1);
    assert!(omdb.request(0).contains("i=tt0133093"));
  }

  #[tokio::test]
  async fn title_search_resolves_when_the_filename_has_no_imdb_id() {
    let tmdb = MockServer::serve_sequence(vec![
      r#"{"results":[{"id":603,"title":"The Matrix","release_date":"1999-03-30"}]}"#.to_string(),
      MOVIE_DETAILS.to_string(),
    ])
    .await;
    let omdb = MockServer::serve_json(OMDB_TITLE.to_string()).await;
    let fetcher = fetcher(Some(&tmdb), Some(&omdb));

    let (warnings, merged, _sources) = fetcher.fetch(&matrix_query()).await.expect("fetch");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(tmdb.request(0).contains("/search/movie"));
    // The IMDb id cross-linked out of the TMDB details reached OMDb.
    assert_eq!(omdb.request_count(), 1);
    assert!(merged.external_ids.iter().any(|e| e.provider == "imdb"));
  }

  #[tokio::test]
  async fn omdb_title_search_is_the_last_resort_without_tmdb() {
    // OMDb serves the search page first, then the title fetch.
    let omdb = MockServer::serve_sequence(vec![
      r#"{"Search":[{"Title":"The Matrix","Year":"1999","imdbID":"tt0133093","Type":"movie"}],"Response":"True"}"#.to_string(),
      OMDB_TITLE.to_string(),
    ])
    .await;
    let fetcher = fetcher(None, Some(&omdb));

    let (warnings, merged, _sources) = fetcher.fetch(&matrix_query()).await.expect("fetch");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(omdb.request_count(), 2);
    assert!(omdb.request(0).contains("s=The"));
    assert_eq!(merged.external_ids.len(), 1);
    assert_eq!(merged.release_year, Some(1999));
  }

  #[tokio::test]
  async fn partial_failure_still_returns_what_worked() {
    let broken_tmdb = MockServer::serve_status(500, "boom").await;
    let omdb = MockServer::serve_json(OMDB_TITLE.to_string()).await;
    let fetcher = fetcher(Some(&broken_tmdb), Some(&omdb));

    let (warnings, merged, _sources) = fetcher
      .fetch(&with_imdb(matrix_query()))
      .await
      .expect("omdb alone is enough");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].starts_with("tmdb:"), "{warnings:?}");
    assert_eq!(merged.release_year, Some(1999));
  }

  #[tokio::test]
  async fn total_failure_reports_every_warning() {
    let broken_tmdb = MockServer::serve_status(500, "boom").await;
    let broken_omdb = MockServer::serve_status(401, "bad key").await;
    let fetcher = fetcher(Some(&broken_tmdb), Some(&broken_omdb));

    let err = fetcher.fetch(&with_imdb(matrix_query())).await.expect_err("all failed");
    assert!(err.contains("tmdb:"), "{err}");
    assert!(err.contains("omdb:"), "{err}");
  }

  #[tokio::test]
  async fn no_match_at_all_is_an_error() {
    let tmdb = MockServer::serve_json(r#"{"results":[]}"#.to_string()).await;
    let omdb = MockServer::serve_json(r#"{"Search":[],"Response":"False","Error":"Movie not found!"}"#.to_string())
      .await;
    let fetcher = fetcher(Some(&tmdb), Some(&omdb));

    let err = fetcher.fetch(&matrix_query()).await.expect_err("nothing matched");
    assert!(err.contains("omdb: no search results"), "{err}");
  }

  #[tokio::test]
  async fn disabled_fetcher_refuses_to_fetch() {
    let fetcher = MetadataFetcher {
      registry: ProviderRegistry::new(),
      tmdb: None,
      omdb: None,
    };
    assert!(!fetcher.enabled());
    let err = fetcher.fetch(&matrix_query()).await.expect_err("no providers");
    assert!(err.contains("no metadata providers configured"), "{err}");
  }

  #[tokio::test]
  async fn merged_text_rows_keep_locale_entries() {
    // The DB write drops all-empty localized entries; the fetch keeps every
    // locale TMDB reported, with its own title/synopsis.
    let tmdb = MockServer::serve_sequence(vec![
      r#"{"results":[{"id":603,"title":"The Matrix","release_date":"1999-03-30"}]}"#.to_string(),
      MOVIE_DETAILS.to_string(),
    ])
    .await;
    let fetcher = fetcher(Some(&tmdb), None);
    let (warnings, merged, _sources) = fetcher.fetch(&matrix_query()).await.expect("fetch");
    assert!(warnings.is_empty());
    let en = merged
      .localized_texts
      .iter()
      .find(|t| t.locale == "en")
      .expect("en");
    assert_eq!(en.title, "The Matrix");
    assert!(!en.synopsis.is_empty());
  }

  // -- plugin registration / resolution ---------------------------------

  use dionysus_metadata::ProviderError;
  use dionysus_metadata_model::{ExternalId, MediaKind, SearchResult};

  /// Stand-in for a `WasmProvider`: the registry cannot tell the
  /// difference, which is exactly what the loop under test relies on.
  struct FakePlugin {
    id: String,
    fail_fetch: bool,
  }

  #[async_trait::async_trait]
  impl MetadataProvider for FakePlugin {
    fn id(&self) -> &str {
      &self.id
    }

    async fn search(&self, _query: &MediaQuery) -> Result<Vec<SearchResult>, ProviderError> {
      Ok(vec![SearchResult {
        id: format!("{}:1", self.id),
        title: "The Matrix".to_string(),
        year: Some(1999),
        kind: MediaKind::Movie,
        score: 0.9,
      }])
    }

    async fn fetch(&self, _id: &str) -> Result<MediaMetadata, ProviderError> {
      if self.fail_fetch {
        return Err(ProviderError::Network("plugin down".to_string()));
      }
      Ok(MediaMetadata {
        release_year: Some(2000), // wrong year — must lose the merge
        external_ids: vec![ExternalId { provider: self.id.clone(), value: "xyz".into() }],
        ..Default::default()
      })
    }
  }

  fn plugin(id: &str) -> Arc<FakePlugin> {
    Arc::new(FakePlugin { id: id.to_string(), fail_fetch: false })
  }

  #[tokio::test]
  async fn plugin_resolves_after_natives_and_only_fills_gaps() {
    let omdb = MockServer::serve_sequence(vec![
      r#"{"Search":[{"Title":"The Matrix","Year":"1999","imdbID":"tt0133093","Type":"movie"}],"Response":"True"}"#.to_string(),
      OMDB_TITLE.to_string(),
    ])
    .await;
    let mut fetcher = fetcher(None, Some(&omdb));
    fetcher.register_plugin(plugin("fixture")).expect("register");

    let (warnings, merged, sources) = fetcher.fetch(&matrix_query()).await.expect("fetch");
    assert!(warnings.is_empty(), "{warnings:?}");

    // OMDb (priority 5) beats the plugin (priority 1) on the shared field…
    assert_eq!(merged.release_year, Some(1999));
    // …but the plugin still contributes what nobody else had.
    assert!(merged.external_ids.iter().any(|e| e.provider == "fixture" && e.value == "xyz"));

    // Both answerers are reported, each with its own descriptor.
    let ids: Vec<&str> = sources.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["omdb", "fixture"]);
  }

  #[tokio::test]
  async fn plugin_only_fetcher_enables_enrichment_without_keys() {
    let mut fetcher = fetcher(None, None);
    assert!(!fetcher.enabled());
    fetcher.register_plugin(plugin("fixture")).expect("register");
    assert!(fetcher.enabled());

    let (warnings, merged, _sources) = fetcher.fetch(&matrix_query()).await.expect("fetch");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(merged.release_year, Some(2000));
  }

  #[tokio::test]
  async fn duplicate_plugin_id_is_rejected() {
    // A native in the registry: its ids are off-limits to plugins too.
    let tmdb = MockServer::serve_json("{}".to_string()).await;
    let mut fetcher = fetcher(Some(&tmdb), None);
    fetcher.register_plugin(plugin("fixture")).expect("first");
    let err = fetcher.register_plugin(plugin("fixture")).expect_err("dup");
    assert!(err.contains("already registered"), "{err}");
    let err = fetcher.register_plugin(plugin("tmdb")).expect_err("native id");
    assert!(err.contains("already registered"), "{err}");
  }

  #[tokio::test]
  async fn failing_plugin_degrades_to_a_warning() {
    let omdb = MockServer::serve_json(OMDB_TITLE.to_string()).await;
    let mut fetcher = fetcher(None, Some(&omdb));
    let broken = FakePlugin { id: "broken".to_string(), fail_fetch: true };
    fetcher.register_plugin(Arc::new(broken)).expect("register");

    let (warnings, merged, _sources) = fetcher
      .fetch(&with_imdb(matrix_query()))
      .await
      .expect("omdb survives");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].starts_with("broken:"), "{warnings:?}");
    assert_eq!(merged.release_year, Some(1999));
  }
}
