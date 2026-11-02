//! IMDb metadata via OMDb (www.omdbapi.com) — IMDb has no official public
//! API, so OMDb is the sanctioned stand-in. It is the secondary provider:
//! TMDB wins field-level merges, OMDb backfills (notably `summary`) and is
//! the direct path when the filename parser found a `tt…` id.

use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;

use dionysus_metadata::{MetadataProvider, ProviderError};
use dionysus_metadata_model::{
  Descriptor, ExternalId, LocalizedText, MediaImageKind, MediaKind, MediaMetadata, MediaQuery,
  RemoteImage, SearchResult,
};

use crate::{get_json, title_score, year_of};

const OMDB_API_BASE: &str = "https://www.omdbapi.com/";
const REQUEST_TIMEOUT_SECS: u64 = 30;

pub struct OmdbProvider {
  client: reqwest::Client,
  base_url: String,
  api_key: String,
}

impl OmdbProvider {
  pub fn new(api_key: impl Into<String>) -> Self {
    let client = reqwest::Client::builder()
      .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
      .build()
      .expect("reqwest client");
    Self {
      client,
      base_url: OMDB_API_BASE.to_string(),
      api_key: api_key.into(),
    }
  }

  /// `OMDB_API_KEY` from the environment; `None` when unset or blank.
  pub fn from_env() -> Option<Self> {
    std::env::var("OMDB_API_KEY")
      .ok()
      .map(|k| k.trim().to_string())
      .filter(|k| !k.is_empty())
      .map(Self::new)
  }

  /// Override the API root — for tests and proxies.
  pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
    self.base_url = base_url.into();
    self
  }

  async fn json<T: serde::de::DeserializeOwned>(
    &self,
    query: &[(&str, String)],
  ) -> Result<T, ProviderError> {
    let mut params: Vec<(&str, String)> = Vec::with_capacity(query.len() + 1);
    params.push(("apikey", self.api_key.clone()));
    params.extend_from_slice(query);
    get_json(&self.client, self.base_url.clone(), &params, None).await
  }
}

#[async_trait]
impl MetadataProvider for OmdbProvider {
  fn id(&self) -> &str {
    "omdb"
  }

  fn descriptor(&self) -> Descriptor {
    Descriptor::new("omdb", "OMDb", env!("CARGO_PKG_VERSION"))
  }

  async fn search(&self, query: &MediaQuery) -> Result<Vec<SearchResult>, ProviderError> {
    if query.title.trim().is_empty() {
      return Ok(Vec::new());
    }
    let mut params: Vec<(&str, String)> = vec![
      ("s", query.title.clone()),
      (
        "type",
        match query.kind {
          MediaKind::Movie => "movie",
          MediaKind::Episode => "series",
        }
        .to_string(),
      ),
    ];
    if let Some(year) = query.year {
      params.push(("y", year.to_string()));
    }
    let page: SearchPage = self.json(&params).await?;
    // "Movie not found!" is a legitimate empty search, not an error.
    if page.is_failure() {
      return Ok(Vec::new());
    }
    let mut results: Vec<SearchResult> = page
      .results
      .into_iter()
      .filter_map(|entry| {
        let title = entry.title.filter(|t| is_set(t))?;
        let imdb_id = entry.imdb_id.filter(|id| is_set(id))?;
        let year = year_of(entry.year.as_deref());
        let score = title_score(&query.title, &title, query.year, year);
        Some(SearchResult {
          id: imdb_id,
          title,
          year,
          kind: query.kind,
          score,
        })
      })
      .collect();
    results.sort_by(|a, b| b.score.total_cmp(&a.score));
    Ok(results)
  }

  async fn fetch(&self, id: &str) -> Result<MediaMetadata, ProviderError> {
    let title: TitleResult = self
      .json(&[("i", id.to_string()), ("plot", "full".to_string())])
      .await?;
    if title.is_failure() {
      return Err(ProviderError::NotFound(format!(
        "omdb {id}: {}",
        title.error.unwrap_or_else(|| "not found".to_string())
      )));
    }

    let kind = match title.media_type.as_deref() {
      Some("series") | Some("episode") => MediaKind::Episode,
      _ => MediaKind::Movie,
    };
    let imdb_id = title
      .imdb_id
      .filter(|v| is_set(v))
      .unwrap_or_else(|| id.to_string());

    // The full OMDb plot fills both text fields: it wins `synopsis` only
    // when TMDB has none, and always backfills `summary`.
    let plot = title.plot.unwrap_or_default();
    let text = LocalizedText {
      locale: "en".to_string(),
      title: title.title.unwrap_or_default(),
      synopsis: plot.clone(),
      summary: plot,
    };

    let external_ids = vec![ExternalId {
      provider: "imdb".to_string(),
      value: imdb_id,
    }];
    let mut images = Vec::new();
    if let Some(poster) = title.poster.as_deref().filter(|p| is_set(p)) {
      images.push(RemoteImage {
        url: poster.to_string(),
        kind: MediaImageKind::Poster,
        locale: None,
      });
    }

    Ok(MediaMetadata {
      release_year: year_of(title.year.as_deref()),
      kind,
      // "136 min" → seconds; series report a per-episode runtime.
      runtime_secs: runtime_secs(title.runtime.as_deref()),
      localized_texts: vec![text],
      images,
      credits: Vec::new(),
      external_ids,
    })
  }
}

// --- response shapes ------------------------------------------------------

#[derive(Deserialize)]
struct SearchPage {
  #[serde(rename = "Search", default)]
  results: Vec<SearchEntry>,
  #[serde(rename = "Response", default)]
  response: Option<String>,
}

impl SearchPage {
  fn is_failure(&self) -> bool {
    self.response.as_deref() == Some("False")
  }
}

#[derive(Deserialize)]
struct SearchEntry {
  #[serde(rename = "Title")]
  title: Option<String>,
  #[serde(rename = "Year")]
  year: Option<String>,
  #[serde(rename = "imdbID")]
  imdb_id: Option<String>,
}

#[derive(Deserialize)]
struct TitleResult {
  #[serde(rename = "Title")]
  title: Option<String>,
  #[serde(rename = "Year")]
  year: Option<String>,
  #[serde(rename = "Runtime")]
  runtime: Option<String>,
  #[serde(rename = "Plot")]
  plot: Option<String>,
  #[serde(rename = "Poster")]
  poster: Option<String>,
  #[serde(rename = "Type")]
  media_type: Option<String>,
  #[serde(rename = "imdbID")]
  imdb_id: Option<String>,
  #[serde(rename = "Response")]
  response: Option<String>,
  #[serde(rename = "Error")]
  error: Option<String>,
}

impl TitleResult {
  fn is_failure(&self) -> bool {
    self.response.as_deref() == Some("False")
  }
}

/// `"136 min"` → `8160`; anything unparsable → `None`.
fn runtime_secs(runtime: Option<&str>) -> Option<i32> {
  let raw = runtime?;
  let minutes: i32 = raw.split_whitespace().next()?.parse().ok()?;
  Some(minutes.saturating_mul(60))
}

/// OMDb/TMDB style placeholders: non-empty and not `"N/A"`.
fn is_set(value: &str) -> bool {
  !value.trim().is_empty() && value.trim() != "N/A"
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::MockServer;

  fn query() -> MediaQuery {
    MediaQuery {
      title: "The Matrix".to_string(),
      year: Some(1999),
      ..Default::default()
    }
  }

  #[tokio::test]
  async fn search_maps_entries_and_orders_by_score() {
    let server = MockServer::serve_json(
      r#"{
        "Search": [
          {"Title": "The Matrix Reloaded", "Year": "2003", "imdbID": "tt0234215", "Type": "movie"},
          {"Title": "The Matrix", "Year": "1999", "imdbID": "tt0133093", "Type": "movie"}
        ],
        "totalResults": "2",
        "Response": "True"
      }"#
      .to_string(),
    )
    .await;
    let omdb = OmdbProvider::new("test-key").with_base_url(server.base_url());

    let results = omdb.search(&query()).await.expect("search");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "tt0133093");
    assert_eq!(results[0].year, Some(1999));
    assert!(results[0].score > results[1].score);

    let request = server.request(0);
    assert!(request.contains("apikey=test-key"), "{request}");
    assert!(request.contains("s=The+Matrix") || request.contains("s=The%20Matrix"), "{request}");
    assert!(request.contains("type=movie"), "{request}");
    assert!(request.contains("y=1999"), "{request}");
  }

  #[tokio::test]
  async fn search_for_series_filters_on_type_series() {
    let server = MockServer::serve_json(r#"{"Search":[],"Response":"False","Error":"Movie not found!"}"#.to_string())
      .await;
    let omdb = OmdbProvider::new("k").with_base_url(server.base_url());
    let query = MediaQuery {
      title: "Breaking Bad".to_string(),
      kind: MediaKind::Episode,
      ..Default::default()
    };
    // "not found" is an empty result set, never an error.
    let results = omdb.search(&query).await.expect("search");
    assert!(results.is_empty());
    assert!(server.request(0).contains("type=series"), "{}", server.request(0));
  }

  #[tokio::test]
  async fn fetch_maps_runtime_year_poster_and_plot() {
    let server = MockServer::serve_json(
      r#"{
        "Title": "The Matrix",
        "Year": "1999",
        "Runtime": "136 min",
        "Plot": "Neo discovers the truth about the Matrix.",
        "Poster": "https://m.media-amazon.com/images/M/poster.jpg",
        "Type": "movie",
        "imdbID": "tt0133093",
        "Response": "True"
      }"#
      .to_string(),
    )
    .await;
    let omdb = OmdbProvider::new("k").with_base_url(server.base_url());

    let meta = omdb.fetch("tt0133093").await.expect("fetch");
    assert_eq!(meta.kind, MediaKind::Movie);
    assert_eq!(meta.release_year, Some(1999));
    assert_eq!(meta.runtime_secs, Some(136 * 60));
    assert_eq!(meta.external_ids, [ExternalId { provider: "imdb".into(), value: "tt0133093".into() }]);

    let text = &meta.localized_texts[0];
    assert_eq!(text.locale, "en");
    assert_eq!(text.title, "The Matrix");
    // OMDb owns both fields; TMDB's synopsis wins the merge when present.
    assert_eq!(text.synopsis, "Neo discovers the truth about the Matrix.");
    assert_eq!(text.summary, text.synopsis);

    assert_eq!(meta.images.len(), 1);
    assert!(meta.images[0].url.starts_with("https://m.media-amazon.com/"));
    assert_eq!(meta.images[0].kind, MediaImageKind::Poster);
  }

  #[tokio::test]
  async fn fetch_treats_na_poster_and_runtime_as_absent() {
    let server = MockServer::serve_json(
      r#"{
        "Title": "Obscure Film", "Year": "N/A", "Runtime": "N/A", "Plot": "P.",
        "Poster": "N/A", "Type": "movie", "imdbID": "tt0000000", "Response": "True"
      }"#
      .to_string(),
    )
    .await;
    let omdb = OmdbProvider::new("k").with_base_url(server.base_url());
    let meta = omdb.fetch("tt0000000").await.expect("fetch");
    assert_eq!(meta.release_year, None);
    assert_eq!(meta.runtime_secs, None);
    assert!(meta.images.is_empty());
  }

  #[tokio::test]
  async fn series_type_maps_to_episode_kind() {
    let server = MockServer::serve_json(
      r#"{
        "Title": "Breaking Bad", "Year": "2008–2013", "Runtime": "45 min",
        "Plot": "A teacher cooks.", "Poster": "N/A", "Type": "series",
        "imdbID": "tt0903747", "Response": "True"
      }"#
      .to_string(),
    )
    .await;
    let omdb = OmdbProvider::new("k").with_base_url(server.base_url());
    let meta = omdb.fetch("tt0903747").await.expect("fetch");
    assert_eq!(meta.kind, MediaKind::Episode);
    assert_eq!(meta.release_year, Some(2008), "series span keeps its first year");
    assert_eq!(meta.runtime_secs, Some(45 * 60));
  }

  #[tokio::test]
  async fn error_response_is_not_found() {
    let server = MockServer::serve_json(r#"{"Response":"False","Error":"Movie not found!"}"#.to_string())
      .await;
    let omdb = OmdbProvider::new("k").with_base_url(server.base_url());
    let err = omdb.fetch("tt9999999").await.expect_err("must be NotFound");
    assert!(matches!(err, ProviderError::NotFound(_)), "{err}");
  }
}
