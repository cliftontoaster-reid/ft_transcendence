//! Native metadata providers: **TMDB** (primary) and **IMDb via OMDb**.
//!
//! Both implement [`dionysus_metadata::MetadataProvider`], so the registry
//! treats them (and, from Phase 6, WASM plugins) identically. Ids are
//! provider-scoped:
//!
//! - TMDB ids are `movie:<id>` / `tv:<id>` — TMDB's movie and tv id spaces
//!   overlap, so [`TmdbProvider::fetch`] requires the prefix (search and
//!   [`resolve_ids`] always produce one).
//! - OMDb ids are IMDb ids (`tt0133093`).
//!
//! Configuration is env-only: `TMDB_API_KEY` (v3 key or v4 bearer token) and
//! `OMDB_API_KEY`. With neither set, the importer skips enrichment entirely.

mod fetch;
mod omdb;
mod resolve;
mod tmdb;

#[cfg(test)]
mod test_util;

pub use fetch::MetadataFetcher;
pub use omdb::OmdbProvider;
pub use resolve::{resolve_ids, ResolvedIds};
pub use tmdb::TmdbProvider;

use serde::de::DeserializeOwned;

use dionysus_metadata::ProviderError;

/// GET `url`, decode JSON, and map transport/status/decode failures onto
/// [`ProviderError`]. Shared by both native providers.
pub(crate) async fn get_json<T: DeserializeOwned>(
  client: &reqwest::Client,
  url: String,
  query: &[(&str, String)],
  bearer: Option<&str>,
) -> Result<T, ProviderError> {
  let mut request = client.get(&url);
  if let Some(token) = bearer {
    request = request.bearer_auth(token);
  }
  if !query.is_empty() {
    request = request.query(query);
  }
  let response = request
    .send()
    .await
    .map_err(|e| ProviderError::Network(format!("{url}: {e}")))?;
  let status = response.status();
  if status == reqwest::StatusCode::NOT_FOUND {
    return Err(ProviderError::NotFound(format!("{url} -> 404")));
  }
  if !status.is_success() {
    return Err(ProviderError::Http { status: status.as_u16(), url });
  }
  response
    .json::<T>()
    .await
    .map_err(|e| ProviderError::Decode(format!("{url}: {e}")))
}

/// Relevance score for providers that return results *ordered* by relevance
/// but carry no score of their own (TMDB and OMDb both work that way).
/// Returns 0.0–1.0, higher first.
pub(crate) fn title_score(
  query_title: &str,
  candidate: &str,
  query_year: Option<i16>,
  candidate_year: Option<i16>,
) -> f32 {
  let normalize = |s: &str| {
    s.to_lowercase()
      .split_whitespace()
      .collect::<Vec<_>>()
      .join(" ")
  };
  let (query, candidate) = (normalize(query_title), normalize(candidate));
  if query.is_empty() {
    return 0.3;
  }
  let mut score: f32 = if query == candidate {
    0.95
  } else if candidate.contains(&query) || query.contains(&candidate) {
    0.7
  } else {
    0.4
  };
  // A year match is a strong secondary signal; ±1 covers release-date slop
  // between the filename and the provider's own date.
  if let (Some(a), Some(b)) = (query_year, candidate_year) {
    if a == b {
      score += 0.04;
    } else if (a - b).abs() <= 1 {
      score += 0.02;
    }
  }
  score.min(1.0)
}

/// Leading `YYYY` of a date-ish string: `"1999-03-30"` → `1999`,
/// `"2019–2023"` (series span) → `2019`, `"1999"` → `1999`.
pub(crate) fn year_of(date: Option<&str>) -> Option<i16> {
  let date = date?;
  let digits: String = date.chars().take_while(|c| c.is_ascii_digit()).collect();
  if digits.len() == 4 {
    digits.parse().ok()
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn score_prefers_exact_title_then_contains_then_fuzzy() {
    let exact = title_score("The Matrix", "The Matrix", Some(1999), Some(1999));
    let partial = title_score("The Matrix", "The Matrix Reloaded", Some(1999), Some(2003));
    let unrelated = title_score("The Matrix", "Finding Nemo", Some(1999), Some(2003));
    assert!(exact > partial, "{exact} must beat {partial}");
    assert!(partial > unrelated, "{partial} must beat {unrelated}");
    assert!(exact <= 1.0);
  }

  #[test]
  fn score_boosts_year_agreement() {
    let same = title_score("Dune", "Dune", Some(2021), Some(2021));
    let near = title_score("Dune", "Dune", Some(2021), Some(2022));
    let far = title_score("Dune", "Dune", Some(2021), Some(1984));
    assert!(same > near);
    assert!(near > far);
    assert!(same <= 1.0);
  }

  #[test]
  fn year_extracts_leading_digits_from_any_separator() {
    assert_eq!(year_of(Some("1999-03-30")), Some(1999));
    assert_eq!(year_of(Some("2008-01-20")), Some(2008));
    assert_eq!(year_of(Some("2019–2023")), Some(2019));
    assert_eq!(year_of(Some("1999")), Some(1999));
    assert_eq!(year_of(Some("N/A")), None);
    assert_eq!(year_of(Some("")), None);
    assert_eq!(year_of(None), None);
  }
}
