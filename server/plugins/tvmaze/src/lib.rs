//! TVMaze example plugin — the reference third-party metadata source.
//!
//! Keyless (TVMaze's API is free and unauthenticated), TV-only, and small
//! enough to read in one sitting:
//!
//! - **search** — `singlesearch/shows?q=…&embed=cast`; a 404 is TVMaze's
//!   "no match" and becomes an empty result list, movies are a definitive
//!   empty (the index is television), and the hit's `score` carries through.
//! - **fetch** — `shows/{id}?embed=cast`; average runtime, premiere year,
//!   the HTML summary (stripped) as synopsis *and* summary, cast as credits,
//!   and the show id plus TVMaze's reported IMDb/TMDB/TVDB ids for
//!   cross-linking.
//!
//! Only [`api`] compiles natively (pure mapping, unit-tested); the `Plugin`
//! impl below is wasm-gated because it uses the HTTP/log capabilities.

// Only the wasm-gated `Plugin` impl below needs the `sdk` alias directly
// (the native test build reaches the model types through `mod api`).
#[cfg(target_arch = "wasm32")]
use dionysus_metadata_sdk as sdk;

#[cfg(any(target_arch = "wasm32", test))]
mod api;

#[cfg(target_arch = "wasm32")]
struct Tvmaze;

#[cfg(target_arch = "wasm32")]
impl sdk::Plugin for Tvmaze {
  fn descriptor() -> sdk::Descriptor {
    api::descriptor()
  }

  fn search(query: &sdk::MediaQuery) -> Result<Vec<sdk::SearchResult>, String> {
    // TVMaze indexes television only — a movie can't exist there.
    if query.kind == sdk::MediaKind::Movie {
      return Ok(Vec::new());
    }
    sdk::log(sdk::log_level::DEBUG, &format!("tvmaze search: {}", query.title));

    let response = sdk::http::get(&api::search_url(&query.title))?;
    // Documented "no match" answer — an empty search, not an error.
    if response.status == 404 {
      return Ok(Vec::new());
    }
    if !(200..300).contains(&response.status) {
      return Err(format!("tvmaze: http {} from search", response.status));
    }
    let show: api::Show = sdk::serde_json::from_slice(&response.body)
      .map_err(|e| format!("tvmaze: search json: {e}"))?;
    Ok(api::to_results(&show))
  }

  fn fetch(id: &str) -> Result<sdk::MediaMetadata, String> {
    // Ids handed out by `search` are TVMaze show ids (bare numbers).
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
      return Err(format!("tvmaze: not a show id: {id:?}"));
    }
    sdk::log(sdk::log_level::DEBUG, &format!("tvmaze fetch: {id}"));
    let show: api::Show = sdk::http::get_json(&api::show_url(id))?;
    Ok(api::to_metadata(&show))
  }
}

#[cfg(target_arch = "wasm32")]
sdk::define_provider!(Tvmaze);
