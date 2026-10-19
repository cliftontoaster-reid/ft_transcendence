//! TMDB (api.themoviedb.org/3) — the primary metadata source: search,
//! details, aggregate credits, images, and translations in one
//! `append_to_response` round trip.

use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;

use dionysus_metadata::{MetadataProvider, ProviderError};
use dionysus_metadata_model::{
  Credit, CreditRole, Descriptor, ExternalId, LocalizedText, MediaImageKind, MediaKind,
  MediaMetadata, MediaQuery, RemoteImage, SearchResult,
};

use crate::{get_json, title_score, year_of};

const TMDB_API_BASE: &str = "https://api.themoviedb.org/3";
/// Images live on a separate CDN host from the API.
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p/original";
const REQUEST_TIMEOUT_SECS: u64 = 30;
/// A popular title can have thousands of images; the `include_image_language`
/// filter narrows the language, these caps bound the download count.
const MAX_POSTERS: usize = 8;
const MAX_BACKDROPS: usize = 4;

pub struct TmdbProvider {
  client: reqwest::Client,
  base_url: String,
  api_key: String,
  /// TMDB v4 access tokens are JWTs (`eyJ…`); they go into an
  /// `Authorization: Bearer` header instead of the v3 `api_key` query param.
  bearer: bool,
}

impl TmdbProvider {
  pub fn new(api_key: impl Into<String>) -> Self {
    let api_key = api_key.into();
    let bearer = api_key.starts_with("eyJ");
    let client = reqwest::Client::builder()
      .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
      .build()
      .expect("reqwest client");
    Self {
      client,
      base_url: TMDB_API_BASE.to_string(),
      api_key,
      bearer,
    }
  }

  /// `TMDB_API_KEY` from the environment; `None` when unset or blank (the
  /// importer then runs with TMDB disabled).
  pub fn from_env() -> Option<Self> {
    std::env::var("TMDB_API_KEY")
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

  /// Cross-link an IMDb id to TMDB: `/find/{imdb_id}`. Prefers the result
  /// matching `kind`, then falls back to the other list — a wrong kind is
  /// better than no link at all.
  pub async fn find_by_imdb(
    &self,
    imdb_id: &str,
    kind: MediaKind,
  ) -> Result<Option<String>, ProviderError> {
    let found: FindResponse = self
      .json(
        &format!("/find/{imdb_id}"),
        &[("external_source", "imdb_id".to_string())],
      )
      .await?;
    let (segment, results) = match kind {
      MediaKind::Movie if !found.movie_results.is_empty() => ("movie", &found.movie_results),
      MediaKind::Episode if !found.tv_results.is_empty() => ("tv", &found.tv_results),
      MediaKind::Movie => ("tv", &found.tv_results),
      MediaKind::Episode => ("movie", &found.movie_results),
    };
    Ok(results.first().map(|r| format!("{segment}:{}", r.id)))
  }

  async fn json<T: serde::de::DeserializeOwned>(
    &self,
    path: &str,
    query: &[(&str, String)],
  ) -> Result<T, ProviderError> {
    let mut params: Vec<(&str, String)> = Vec::with_capacity(query.len() + 1);
    let bearer = if self.bearer {
      Some(self.api_key.as_str())
    } else {
      params.push(("api_key", self.api_key.clone()));
      None
    };
    params.extend_from_slice(query);
    get_json(&self.client, format!("{}{path}", self.base_url), &params, bearer).await
  }

  async fn fetch_movie(&self, id: &str) -> Result<MediaMetadata, ProviderError> {
    let movie: MovieDetails = self
      .json(
        &format!("/movie/{id}"),
        &[
          ("append_to_response", "credits,translations,images".to_string()),
          ("include_image_language", "en,null".to_string()),
        ],
      )
      .await?;

    let title = movie.title.clone().unwrap_or_default();
    let original_language = movie.original_language.clone().unwrap_or_default();
    let mut external_ids = vec![ExternalId {
      provider: "tmdb".to_string(),
      value: id.to_string(),
    }];
    if let Some(imdb) = movie.imdb_id.as_deref().filter(|v| is_set(v)) {
      external_ids.push(ExternalId {
        provider: "imdb".to_string(),
        value: imdb.to_string(),
      });
    }
    let release_year = year_of(movie.release_date.as_deref());

    Ok(MediaMetadata {
      release_year,
      kind: MediaKind::Movie,
      runtime_secs: movie.runtime.map(|m| m.saturating_mul(60) as i32),
      localized_texts: translation_texts(
        &movie.translations,
        &original_language,
        &title,
        movie.overview.as_deref().unwrap_or_default(),
      ),
      images: collect_images(&movie.images),
      credits: collect_credits(&movie.credits),
      external_ids,
    })
  }

  async fn fetch_tv(&self, id: &str) -> Result<MediaMetadata, ProviderError> {
    let show: TvDetails = self
      .json(
        &format!("/tv/{id}"),
        &[
          (
            "append_to_response",
            "credits,translations,images,external_ids".to_string(),
          ),
          ("include_image_language", "en,null".to_string()),
        ],
      )
      .await?;

    let title = show.name.clone().unwrap_or_default();
    let original_language = show.original_language.clone().unwrap_or_default();
    let mut external_ids = vec![ExternalId {
      provider: "tmdb".to_string(),
      value: id.to_string(),
    }];
    if let Some(imdb) = show
      .external_ids
      .as_ref()
      .and_then(|e| e.imdb_id.as_deref())
      .filter(|v| is_set(v))
    {
      external_ids.push(ExternalId {
        provider: "imdb".to_string(),
        value: imdb.to_string(),
      });
    }
    let release_year = year_of(show.first_air_date.as_deref());

    Ok(MediaMetadata {
      release_year,
      kind: MediaKind::Episode,
      // Series-level runtime: the first episode slot's length. Often absent
      // on newer TMDB records, in which case we report nothing.
      runtime_secs: show
        .episode_run_time
        .as_ref()
        .and_then(|slots| slots.first())
        .map(|m| m.saturating_mul(60) as i32),
      localized_texts: translation_texts(
        &show.translations,
        &original_language,
        &title,
        show.overview.as_deref().unwrap_or_default(),
      ),
      images: collect_images(&show.images),
      credits: collect_credits(&show.credits),
      external_ids,
    })
  }
}

#[async_trait]
impl MetadataProvider for TmdbProvider {
  fn id(&self) -> &str {
    "tmdb"
  }

  fn descriptor(&self) -> Descriptor {
    Descriptor::new("tmdb", "TMDB", env!("CARGO_PKG_VERSION"))
  }

  async fn search(&self, query: &MediaQuery) -> Result<Vec<SearchResult>, ProviderError> {
    if query.title.trim().is_empty() {
      return Ok(Vec::new());
    }
    let year = query.year.map(|y| y.to_string());
    let mut results: Vec<SearchResult> = match query.kind {
      MediaKind::Movie => {
        let mut params: Vec<(&str, String)> = vec![("query", query.title.clone())];
        if let Some(y) = &year {
          params.push(("year", y.clone()));
        }
        let page: SearchMoviePage = self.json("/search/movie", &params).await?;
        page.results
          .into_iter()
          .filter_map(|r| {
            let title = r.title?;
            let year = year_of(r.release_date.as_deref());
            let score = title_score(&query.title, &title, query.year, year);
            Some(SearchResult {
              id: format!("movie:{}", r.id),
              title,
              year,
              kind: MediaKind::Movie,
              score,
            })
          })
          .collect()
      }
      MediaKind::Episode => {
        let mut params: Vec<(&str, String)> = vec![("query", query.title.clone())];
        if let Some(y) = &year {
          params.push(("first_air_date_year", y.clone()));
        }
        let page: SearchTvPage = self.json("/search/tv", &params).await?;
        page.results
          .into_iter()
          .filter_map(|r| {
            let title = r.name?;
            let year = year_of(r.first_air_date.as_deref());
            let score = title_score(&query.title, &title, query.year, year);
            Some(SearchResult {
              id: format!("tv:{}", r.id),
              title,
              year,
              kind: MediaKind::Episode,
              score,
            })
          })
          .collect()
      }
    };
    // TMDB returns relevance-ordered hits; our own score decides the order
    // the caller sees.
    results.sort_by(|a, b| b.score.total_cmp(&a.score));
    Ok(results)
  }

  async fn fetch(&self, id: &str) -> Result<MediaMetadata, ProviderError> {
    let (segment, tmdb_id) = id.split_once(':').ok_or_else(|| {
      ProviderError::Other(format!(
        "tmdb id {id:?} must be movie:<id> or tv:<id> (ids come from tmdb search/find)"
      ))
    })?;
    match segment {
      "movie" => self.fetch_movie(tmdb_id).await,
      "tv" => self.fetch_tv(tmdb_id).await,
      other => Err(ProviderError::Other(format!(
        "unknown tmdb id segment {other:?} in {id:?}"
      ))),
    }
  }
}

// --- response shapes ------------------------------------------------------

#[derive(Deserialize)]
struct SearchMoviePage {
  #[serde(default)]
  results: Vec<SearchMovie>,
}

#[derive(Deserialize)]
struct SearchMovie {
  id: i64,
  title: Option<String>,
  release_date: Option<String>,
}

#[derive(Deserialize)]
struct SearchTvPage {
  #[serde(default)]
  results: Vec<SearchTv>,
}

#[derive(Deserialize)]
struct SearchTv {
  id: i64,
  name: Option<String>,
  first_air_date: Option<String>,
}

#[derive(Deserialize, Default)]
struct FindResponse {
  #[serde(default)]
  movie_results: Vec<FindResult>,
  #[serde(default)]
  tv_results: Vec<FindResult>,
}

#[derive(Deserialize)]
struct FindResult {
  id: i64,
}

#[derive(Deserialize)]
struct MovieDetails {
  title: Option<String>,
  overview: Option<String>,
  release_date: Option<String>,
  runtime: Option<i64>,
  original_language: Option<String>,
  imdb_id: Option<String>,
  #[serde(default)]
  credits: Credits,
  #[serde(default)]
  translations: Translations,
  #[serde(default)]
  images: Images,
}

#[derive(Deserialize)]
struct TvDetails {
  name: Option<String>,
  overview: Option<String>,
  first_air_date: Option<String>,
  original_language: Option<String>,
  episode_run_time: Option<Vec<i64>>,
  external_ids: Option<ExternalIds>,
  #[serde(default)]
  credits: Credits,
  #[serde(default)]
  translations: Translations,
  #[serde(default)]
  images: Images,
}

#[derive(Deserialize)]
struct ExternalIds {
  imdb_id: Option<String>,
}

#[derive(Deserialize, Default)]
struct Credits {
  #[serde(default)]
  cast: Vec<CastMember>,
  #[serde(default)]
  crew: Vec<CrewMember>,
}

#[derive(Deserialize)]
struct CastMember {
  id: i64,
  name: Option<String>,
  character: Option<String>,
  order: Option<i32>,
}

#[derive(Deserialize)]
struct CrewMember {
  id: i64,
  name: Option<String>,
  job: Option<String>,
  order: Option<i32>,
}

#[derive(Deserialize, Default)]
struct Translations {
  #[serde(default)]
  translations: Vec<Translation>,
}

#[derive(Deserialize)]
struct Translation {
  iso_639_1: Option<String>,
  data: TranslationData,
}

#[derive(Deserialize, Default)]
struct TranslationData {
  title: Option<String>,
  overview: Option<String>,
}

#[derive(Deserialize, Default)]
struct Images {
  #[serde(default)]
  posters: Vec<TmdbImage>,
  #[serde(default)]
  backdrops: Vec<TmdbImage>,
}

#[derive(Deserialize)]
struct TmdbImage {
  file_path: Option<String>,
  iso_639_1: Option<String>,
  vote_average: Option<f64>,
}

// --- mapping --------------------------------------------------------------

/// Per-locale titles/overviews from `translations`, falling back to the
/// detail response itself when TMDB returns none. `summary` stays empty:
/// the OMDb provider owns that field (per-field merge in the registry).
fn translation_texts(
  translations: &Translations,
  original_language: &str,
  fallback_title: &str,
  fallback_synopsis: &str,
) -> Vec<LocalizedText> {
  let mut texts: Vec<LocalizedText> = translations
    .translations
    .iter()
    .filter_map(|t| {
      let title = t.data.title.clone().unwrap_or_default();
      let synopsis = t.data.overview.clone().unwrap_or_default();
      if title.is_empty() && synopsis.is_empty() {
        return None;
      }
      let locale = match t.iso_639_1.as_deref() {
        Some(l) if !l.is_empty() => l.to_string(),
        _ => "und".to_string(),
      };
      Some(LocalizedText {
        locale,
        title,
        synopsis,
        summary: String::new(),
      })
    })
    .collect();

  if texts.is_empty() && (!fallback_title.is_empty() || !fallback_synopsis.is_empty()) {
    let locale = if original_language.is_empty() {
      "und".to_string()
    } else {
      original_language.to_string()
    };
    texts.push(LocalizedText {
      locale,
      title: fallback_title.to_string(),
      synopsis: fallback_synopsis.to_string(),
      summary: String::new(),
    });
  }
  texts
}

fn collect_images(images: &Images) -> Vec<RemoteImage> {
  let mut out = Vec::new();
  push_images(&mut out, &images.posters, MediaImageKind::Poster, MAX_POSTERS);
  push_images(&mut out, &images.backdrops, MediaImageKind::Backdrop, MAX_BACKDROPS);
  out
}

fn push_images(
  out: &mut Vec<RemoteImage>,
  images: &[TmdbImage],
  kind: MediaImageKind,
  cap: usize,
) {
  let mut usable: Vec<&TmdbImage> = images
    .iter()
    .filter(|i| i.file_path.as_deref().is_some_and(|p| p.starts_with('/')))
    .collect();
  // Best-voted first, so the cap drops the long tail.
  usable.sort_by(|a, b| {
    b.vote_average.unwrap_or(0.0)
      .total_cmp(&a.vote_average.unwrap_or(0.0))
  });
  for image in usable.into_iter().take(cap) {
    let path = image.file_path.as_deref().unwrap_or_default();
    out.push(RemoteImage {
      url: format!("{TMDB_IMAGE_BASE}{path}"),
      kind,
      locale: image
        .iso_639_1
        .as_deref()
        .filter(|l| !l.is_empty())
        .map(str::to_string),
    });
  }
}

/// Cast first (so actors hold the low billing numbers), then crew; people
/// credited in several jobs collapse into one [`Credit`] with unioned roles,
/// mirroring the `credit.proto` roles[] convention.
fn collect_credits(credits: &Credits) -> Vec<Credit> {
  let mut out: Vec<Credit> = Vec::new();
  for member in &credits.cast {
    let Some(name) = member.name.as_deref().filter(|n| is_set(n)) else {
      continue;
    };
    push_credit(
      &mut out,
      Credit {
        person_external_ids: vec![ExternalId {
          provider: "tmdb".to_string(),
          value: member.id.to_string(),
        }],
        name: name.to_string(),
        roles: vec![CreditRole::Actor],
        characters: member
          .character
          .as_deref()
          .filter(|c| is_set(c))
          .map(|c| vec![c.to_string()])
          .unwrap_or_default(),
        billing: member.order.unwrap_or(0),
      },
    );
  }
  for member in &credits.crew {
    let Some(role) = member.job.as_deref().and_then(crew_role) else {
      continue;
    };
    let Some(name) = member.name.as_deref().filter(|n| is_set(n)) else {
      continue;
    };
    push_credit(
      &mut out,
      Credit {
        person_external_ids: vec![ExternalId {
          provider: "tmdb".to_string(),
          value: member.id.to_string(),
        }],
        name: name.to_string(),
        roles: vec![role],
        characters: Vec::new(),
        billing: member.order.unwrap_or(0),
      },
    );
  }
  out
}

fn push_credit(out: &mut Vec<Credit>, credit: Credit) {
  let Some(existing) = out
    .iter_mut()
    .find(|c| c.person_external_ids == credit.person_external_ids)
  else {
    out.push(credit);
    return;
  };
  for role in credit.roles {
    if !existing.roles.contains(&role) {
      existing.roles.push(role);
    }
  }
  for character in credit.characters {
    if !existing.characters.contains(&character) {
      existing.characters.push(character);
    }
  }
  existing.billing = existing.billing.min(credit.billing);
}

/// TMDB crew job → `credit.proto` role vocabulary. Jobs outside the
/// vocabulary (departments we don't model) are skipped rather than coerced.
fn crew_role(job: &str) -> Option<CreditRole> {
  Some(match job {
    "Director" => CreditRole::Director,
    "Assistant Director" | "First Assistant Director" => CreditRole::AssistantDirector,
    "Writer" | "Teleplay" => CreditRole::Writer,
    "Screenplay" => CreditRole::ScreenplayWriter,
    "Story" => CreditRole::StoryWriter,
    "Producer" => CreditRole::Producer,
    "Executive Producer" => CreditRole::ExecutiveProducer,
    "Co-Producer" => CreditRole::CoProducer,
    "Line Producer" => CreditRole::LineProducer,
    "Casting" => CreditRole::CastingDirector,
    "Director of Photography" => CreditRole::Cinematographer,
    "Editor" => CreditRole::Editor,
    "Production Design" => CreditRole::ProductionDesigner,
    "Art Direction" | "Art Director" => CreditRole::ArtDirector,
    "Set Decoration" => CreditRole::SetDecorator,
    "Costume Design" => CreditRole::CostumeDesigner,
    "Makeup" | "Makeup Artist" => CreditRole::MakeupArtist,
    "Hairstyling" | "Hair" => CreditRole::HairStylist,
    "Original Music Composer" | "Music" => CreditRole::Composer,
    "Music Supervisor" => CreditRole::MusicSupervisor,
    "Sound Design" | "Sound" => CreditRole::SoundDesigner,
    "Visual Effects" => CreditRole::VisualEffectsArtist,
    "Stunt Coordinator" => CreditRole::Coordinator,
    "Choreography" | "Choreographer" => CreditRole::Choreographer,
    _ => return None,
  })
}

/// OMDb/TMDB style placeholders: non-empty and not `"N/A"`.
fn is_set(value: &str) -> bool {
  !value.trim().is_empty() && value.trim() != "N/A"
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

  #[tokio::test]
  async fn search_maps_hits_with_scores_and_prefixed_ids() {
    let server = MockServer::serve_json(
      r#"{"page":1,"results":[
        {"id":604,"title":"The Matrix Reloaded","release_date":"2003-05-15"},
        {"id":603,"title":"The Matrix","release_date":"1999-03-30"}
      ]}"#
      .to_string(),
    )
    .await;
    let tmdb = TmdbProvider::new("test-key").with_base_url(server.base_url());

    let results = tmdb.search(&matrix_query()).await.expect("search");
    assert_eq!(results.len(), 2);
    // Exact title + year match must rank above the sequel.
    assert_eq!(results[0].id, "movie:603");
    assert_eq!(results[0].title, "The Matrix");
    assert_eq!(results[0].year, Some(1999));
    assert!(results[0].score > results[1].score);
    assert_eq!(results[1].id, "movie:604");

    // v3 auth goes in the query string; the year hint rides along.
    let request = server.request(0);
    assert!(request.starts_with("GET /search/movie?"), "{request}");
    assert!(request.contains("api_key=test-key"), "{request}");
    assert!(request.contains("year=1999"), "{request}");
  }

  #[tokio::test]
  async fn search_for_episodes_queries_the_tv_endpoint() {
    let server = MockServer::serve_json(
      r#"{"page":1,"results":[{"id":1396,"name":"Breaking Bad","first_air_date":"2008-01-20"}]}"#
        .to_string(),
    )
    .await;
    let tmdb = TmdbProvider::new("test-key").with_base_url(server.base_url());

    let query = MediaQuery {
      title: "Breaking Bad".to_string(),
      year: Some(2008),
      kind: MediaKind::Episode,
      season: Some(3),
      episode: Some(7),
      ..Default::default()
    };
    let results = tmdb.search(&query).await.expect("search");
    assert_eq!(results[0].id, "tv:1396");
    assert_eq!(results[0].kind, MediaKind::Episode);
    assert!(server.request(0).starts_with("GET /search/tv?"), "{}", server.request(0));
    assert!(server.request(0).contains("first_air_date_year=2008"));
  }

  #[tokio::test]
  async fn fetch_movie_maps_texts_images_credits_and_ids() {
    let server = MockServer::serve_json(
      r#"{
        "id": 603,
        "title": "The Matrix",
        "overview": "A hacker learns the truth about reality.",
        "release_date": "1999-03-30",
        "runtime": 136,
        "original_language": "en",
        "imdb_id": "tt0133093",
        "credits": {
          "cast": [
            {"id": 6384, "name": "Keanu Reeves", "character": "Neo", "order": 0},
            {"id": 113, "name": "Hugo Weaving", "character": "Agent Smith", "order": 4}
          ],
          "crew": [
            {"id": 9339, "name": "Lana Wachowski", "job": "Director", "order": 0},
            {"id": 9339, "name": "Lana Wachowski", "job": "Screenplay", "order": 0},
            {"id": 9339, "name": "Lana Wachowski", "job": "Unmodelled Job", "order": 0},
            {"id": 555, "name": "Don Davis", "job": "Original Music Composer", "order": 0}
          ]
        },
        "translations": {"translations": [
          {"iso_639_1": "en", "data": {"title": "The Matrix", "overview": "A hacker learns the truth about reality."}},
          {"iso_639_1": "fr", "data": {"title": "Matrix", "overview": ""}},
          {"iso_639_1": "de", "data": {"title": "", "overview": ""}}
        ]},
        "images": {
          "posters": [
            {"file_path": "/popular.jpg", "iso_639_1": "en", "vote_average": 9.0},
            {"file_path": "/obscure.jpg", "iso_639_1": "en", "vote_average": 1.0},
            {"file_path": null, "iso_639_1": null}
          ],
          "backdrops": [{"file_path": "/wide.jpg", "iso_639_1": null}]
        }
      }"#
      .to_string(),
    )
    .await;
    let tmdb = TmdbProvider::new("test-key").with_base_url(server.base_url());

    let meta = tmdb.fetch("movie:603").await.expect("fetch");
    assert_eq!(meta.kind, MediaKind::Movie);
    assert_eq!(meta.release_year, Some(1999));
    assert_eq!(meta.runtime_secs, Some(136 * 60));

    // External ids: tmdb always, imdb when the record carries one.
    assert!(meta.external_ids.iter().any(|e| e.provider == "tmdb" && e.value == "603"));
    assert!(meta.external_ids.iter().any(|e| e.provider == "imdb" && e.value == "tt0133093"));

    // The empty German entry is dropped; en/fr survive with synopsis only.
    let locales: Vec<&str> = meta.localized_texts.iter().map(|t| t.locale.as_str()).collect();
    assert_eq!(locales, ["en", "fr"]);
    assert_eq!(meta.localized_texts[0].title, "The Matrix");
    assert!(meta.localized_texts[0].synopsis.contains("hacker"));
    assert!(meta.localized_texts[0].summary.is_empty(), "OMDb owns the summary field");

    // Posters sorted by votes (popular first), null path dropped, URL built
    // against the CDN host.
    let posters: Vec<&RemoteImage> = meta
      .images
      .iter()
      .filter(|i| i.kind == MediaImageKind::Poster)
      .collect();
    assert_eq!(posters.len(), 2);
    assert!(posters[0].url.ends_with("/popular.jpg"));
    assert!(posters[1].url.ends_with("/obscure.jpg"));
    let backdrop = meta
      .images
      .iter()
      .find(|i| i.kind == MediaImageKind::Backdrop)
      .expect("backdrop");
    assert_eq!(backdrop.url, "https://image.tmdb.org/t/p/original/wide.jpg");
    assert_eq!(backdrop.locale, None, "untagged image has no locale");

    // Two crew jobs for the same person collapse into one credit.
    let lana = meta
      .credits
      .iter()
      .find(|c| c.name == "Lana Wachowski")
      .expect("lana");
    assert_eq!(lana.roles, [CreditRole::Director, CreditRole::ScreenplayWriter]);
    assert!(lana.person_external_ids.iter().any(|e| e.value == "9339"));
    let keanu = meta.credits.iter().find(|c| c.name == "Keanu Reeves").expect("keanu");
    assert_eq!(keanu.roles, [CreditRole::Actor]);
    assert_eq!(keanu.characters, ["Neo"]);
    assert_eq!(keanu.billing, 0);
    assert!(meta.credits.iter().any(|c| c.name == "Don Davis"));
    assert!(!meta.credits.iter().any(|c| c.name == "Lana Wachowski" && c.roles.len() == 1));
    assert!(!meta.credits.iter().any(|c| c.name.is_empty()));
  }

  #[tokio::test]
  async fn fetch_tv_reads_imdb_id_from_external_ids() {
    let server = MockServer::serve_json(
      r#"{
        "id": 1396,
        "name": "Breaking Bad",
        "overview": "A chemistry teacher turns to crime.",
        "first_air_date": "2008-01-20",
        "original_language": "en",
        "episode_run_time": [47],
        "external_ids": {"imdb_id": "tt0903747", "tvdb_id": 81189},
        "credits": {"cast": [], "crew": []},
        "translations": {"translations": []},
        "images": {"posters": [], "backdrops": []}
      }"#
      .to_string(),
    )
    .await;
    let tmdb = TmdbProvider::new("test-key").with_base_url(server.base_url());

    let meta = tmdb.fetch("tv:1396").await.expect("fetch");
    assert_eq!(meta.kind, MediaKind::Episode);
    assert_eq!(meta.release_year, Some(2008));
    assert_eq!(meta.runtime_secs, Some(47 * 60));
    assert!(meta.external_ids.iter().any(|e| e.provider == "tmdb" && e.value == "1396"));
    assert!(meta.external_ids.iter().any(|e| e.provider == "imdb" && e.value == "tt0903747"));
    // No translations array → fallback to the detail response itself.
    assert_eq!(meta.localized_texts.len(), 1);
    assert_eq!(meta.localized_texts[0].locale, "en");
    assert_eq!(meta.localized_texts[0].title, "Breaking Bad");
  }

  #[tokio::test]
  async fn fetch_rejects_unprefixed_ids() {
    let tmdb = TmdbProvider::new("test-key").with_base_url("http://127.0.0.1:1");
    let err = tmdb.fetch("603").await.expect_err("must reject");
    assert!(matches!(err, ProviderError::Other(_)), "{err}");
    let err = tmdb.fetch("game:603").await.expect_err("must reject");
    assert!(matches!(err, ProviderError::Other(_)), "{err}");
  }

  #[tokio::test]
  async fn find_by_imdb_prefers_the_requested_kind_then_falls_back() {
    let server = MockServer::serve_json(
      r#"{"movie_results":[{"id":603}],"tv_results":[{"id":1396}]}"#.to_string(),
    )
    .await;
    let tmdb = TmdbProvider::new("test-key").with_base_url(server.base_url());

    let movie = tmdb
      .find_by_imdb("tt0133093", MediaKind::Movie)
      .await
      .expect("find")
      .expect("result");
    assert_eq!(movie, "movie:603");
    let episode = tmdb
      .find_by_imdb("tt0903747", MediaKind::Episode)
      .await
      .expect("find")
      .expect("result");
    assert_eq!(episode, "tv:1396");
    assert!(server.request(0).starts_with("GET /find/tt0133093?"), "{}", server.request(0));
    assert!(server.request(0).contains("external_source=imdb_id"));
  }

  #[tokio::test]
  async fn find_by_imdb_falls_back_to_the_other_list() {
    let server = MockServer::serve_json(r#"{"movie_results":[],"tv_results":[{"id":42}]}"#.to_string())
      .await;
    let tmdb = TmdbProvider::new("test-key").with_base_url(server.base_url());
    // Misparsed as a movie, but the id only exists as a TV show.
    let found = tmdb
      .find_by_imdb("tt0000001", MediaKind::Movie)
      .await
      .expect("find")
      .expect("result");
    assert_eq!(found, "tv:42");
  }

  #[tokio::test]
  async fn bearer_tokens_go_into_the_authorization_header() {
    let server = MockServer::serve_json(r#"{"results":[]}"#.to_string()).await;
    let token = "eyJhbGciOiJIUzI1NiJ9.fake.payload";
    let tmdb = TmdbProvider::new(token).with_base_url(server.base_url());
    tmdb.search(&matrix_query()).await.expect("search");
    let request = server.request(0);
    assert!(request.starts_with("GET /search/movie?"), "{request}");
    assert!(!request.contains("api_key="), "v4 tokens are not query params: {request}");
    assert!(
      request.to_lowercase().contains("authorization: bearer"),
      "{request}"
    );
  }

  #[tokio::test]
  async fn http_and_decode_errors_map_to_provider_errors() {
    let missing = MockServer::serve_status(404, "").await;
    let tmdb = TmdbProvider::new("k").with_base_url(missing.base_url());
    let err = tmdb.fetch("movie:1").await.expect_err("404");
    assert!(matches!(err, ProviderError::NotFound(_)), "{err}");

    let broken = MockServer::serve_json("not json at all".to_string()).await;
    let tmdb = TmdbProvider::new("k").with_base_url(broken.base_url());
    let err = tmdb.fetch("movie:1").await.expect_err("decode");
    assert!(matches!(err, ProviderError::Decode(_)), "{err}");

    let failing = MockServer::serve_status(500, "boom").await;
    let tmdb = TmdbProvider::new("k").with_base_url(failing.base_url());
    let err = tmdb.fetch("movie:1").await.expect_err("500");
    assert!(matches!(err, ProviderError::Http { status: 500, .. }), "{err}");
  }
}
