//! Test fixture plugin for `dionysus_metadata_wasm`'s integration tests.
//!
//! Behaviors (driven by the query title / fetched id) so one tiny plugin can
//! exercise every host path:
//!
//! - `title` starting with `http` → performs a host-mediated HTTP GET and
//!   reports the status/body, which lets tests point it at a local mock
//!   server (allowed) or a non-allow-listed host (denied).
//! - `title == "fail-search"` / `id == "err"` → returns a guest error.
//! - anything else → canned results/metadata covering every model field.
//!
//! Like every plugin, this crate is a `cdylib` built for `wasm32-wasip2`;
//! on native targets the crate compiles to an empty shell.
#![cfg(target_arch = "wasm32")]

use dionysus_metadata_sdk as sdk;

struct Fixture;

impl sdk::Plugin for Fixture {
  fn descriptor() -> sdk::Descriptor {
    sdk::Descriptor::new("fixture", "Fixture Plugin", env!("CARGO_PKG_VERSION"))
  }

  fn search(query: &sdk::MediaQuery) -> Result<Vec<sdk::SearchResult>, String> {
    sdk::log(sdk::log_level::INFO, &format!("fixture search for {:?}", query.title));

    if query.title.starts_with("http") {
      let response = sdk::http::get(&query.title)?;
      return Ok(vec![sdk::SearchResult {
        id: format!("http:{}", response.status),
        title: format!("{} {}", response.status, preview(&response.body)),
        year: None,
        kind: sdk::MediaKind::Movie,
        score: 1.0,
      }]);
    }

    if query.title == "fail-search" {
      return Err("fixture search failure".to_string());
    }

    Ok(vec![
      sdk::SearchResult {
        id: "fixture:1".to_string(),
        title: query.title.clone(),
        year: query.year,
        kind: query.kind,
        score: 0.9,
      },
      sdk::SearchResult {
        id: "fixture:2".to_string(),
        title: format!("{} (alternate)", query.title),
        year: query.year,
        kind: query.kind,
        score: 0.5,
      },
    ])
  }

  fn fetch(id: &str) -> Result<sdk::MediaMetadata, String> {
    sdk::log(sdk::log_level::DEBUG, &format!("fixture fetch for {id}"));

    if id.starts_with("http") {
      let response = sdk::http::get(id)?;
      return Ok(sdk::MediaMetadata {
        localized_texts: vec![sdk::LocalizedText {
          locale: "en".to_string(),
          title: format!("http {}", response.status),
          synopsis: preview(&response.body),
          ..Default::default()
        }],
        ..Default::default()
      });
    }

    match id {
      "err" => Err("fixture fetch failure".to_string()),
      "fixture:1" => Ok(sdk::MediaMetadata {
        release_year: Some(1999),
        kind: sdk::MediaKind::Episode,
        runtime_secs: Some(5_400),
        localized_texts: vec![sdk::LocalizedText {
          locale: "en".to_string(),
          title: "Fixture Show".to_string(),
          synopsis: "A fixture synopsis.".to_string(),
          summary: "A fixture summary.".to_string(),
        }],
        images: vec![sdk::RemoteImage {
          url: "https://image.tmdb.org/t/p/w500/fixture.jpg".to_string(),
          kind: sdk::MediaImageKind::Poster,
          locale: None,
        }],
        credits: vec![sdk::Credit {
          person_external_ids: vec![sdk::ExternalId {
            provider: "fixture".to_string(),
            value: "7".to_string(),
          }],
          name: "Ada Example".to_string(),
          roles: vec![sdk::CreditRole::Actor],
          characters: vec!["Self".to_string()],
          billing: 0,
        }],
        external_ids: vec![
          sdk::ExternalId { provider: "fixture".to_string(), value: "1".to_string() },
          sdk::ExternalId { provider: "imdb".to_string(), value: "tt0000001".to_string() },
        ],
      }),
      other => Err(format!("unknown fixture id {other}")),
    }
  }
}

/// First few bytes of a body, lossily, for assertion-friendly result titles.
fn preview(body: &[u8]) -> String {
  String::from_utf8_lossy(&body[..body.len().min(32)]).to_string()
}

sdk::define_provider!(Fixture);
