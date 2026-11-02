//! Guest-side SDK for `dionysus:metadata` WASM plugins.
//!
//! A plugin is a `cdylib` compiled for `wasm32-wasip2` that implements
//! [`Plugin`] and invokes [`define_provider!`] once:
//!
//! ```ignore
//! use dionysus_metadata_sdk as sdk;
//!
//! struct MyPlugin;
//!
//! impl sdk::Plugin for MyPlugin {
//!   fn descriptor() -> sdk::Descriptor {
//!     sdk::Descriptor::new("mysource", "My Source", env!("CARGO_PKG_VERSION"))
//!   }
//!
//!   fn search(query: &sdk::MediaQuery) -> Result<Vec<sdk::SearchResult>, String> {
//!     let page: MyApi = sdk::http::get_json(&format!("https://api.example/show?q={}", &query.title))?;
//!     Ok(page.results.into_iter().map(|r| sdk::SearchResult { id: r.id, .. }).collect())
//!   }
//!
//!   fn fetch(id: &str) -> Result<sdk::MediaMetadata, String> {
//!     /* load full metadata via sdk::http */
//!     # unimplemented!()
//!   }
//! }
//!
//! sdk::define_provider!(MyPlugin);
//! ```
//!
//! Design notes:
//!
//! - **All payload types cross the WASM boundary as MessagePack with named
//!   maps** — see [`codec`]; plugin authors never touch raw bytes.
//! - **The guest API is synchronous.** WIT functions have no async on the
//!   guest side; the *host* may run capability calls (HTTP) asynchronously
//!   underneath, and the guest call simply waits for the result.
//! - The single `wit_bindgen::generate!` lives in this crate
//!   (`pub_export_macro: true`); [`define_provider!`] expands the export
//!   glue into the plugin crate against *our* bindings, so a plugin only
//!   depends on this crate — no WIT files, no cargo-component.
//! - On non-wasm targets this crate compiles to just the [`codec`] module
//!   (the boundary encoding is host- and guest-agnostic and tested natively).

pub mod codec;

pub use dionysus_metadata_model as model;
pub use dionysus_metadata_model::{
  Credit, CreditRole, Descriptor, ExternalId, LocalizedText, MediaImageKind, MediaKind,
  MediaMetadata, MediaQuery, RemoteImage, SearchResult, MODEL_VERSION,
};

// Re-exported for hand-rolled decoding by plugin authors
// (`sdk::serde_json::from_slice`, `serde_json::Value` in wire types).
// NOTE: `#[derive(Deserialize)]` expands to paths under the crate name
// `serde`, which only resolves when the *plugin* lists `serde` in its own
// Cargo.toml — a re-export cannot satisfy the derive.
pub use serde;
pub use serde_json;

/// Levels for [`log`], mirroring the order the host maps onto `tracing`.
pub mod log_level {
  pub const ERROR: u8 = 0;
  pub const WARN: u8 = 1;
  pub const INFO: u8 = 2;
  pub const DEBUG: u8 = 3;
  pub const TRACE: u8 = 4;
}

/// What a plugin implements. All methods are plain functions: a component
/// has no constructor, and plugins are stateless units (stateful plugins
/// keep their own `static` items — single-threaded wasm makes that safe).
pub trait Plugin {
  /// Identity + contract version. Build with [`Descriptor::new`] so the
  /// host can detect a model-version mismatch.
  fn descriptor() -> Descriptor;

  /// Resolve a query to provider-scoped candidate ids, best score first.
  fn search(query: &MediaQuery) -> Result<Vec<SearchResult>, String>;

  /// Load full metadata for one of the ids handed out by [`search`](Self::search).
  fn fetch(id: &str) -> Result<MediaMetadata, String>;
}

#[cfg(target_arch = "wasm32")]
wit_bindgen::generate!({
  world: "metadata-provider",
  path: "../../../proto/wit/dionysus/metadata",
  pub_export_macro: true,
});

#[cfg(target_arch = "wasm32")]
pub use exports::dionysus::metadata::provider::Guest;

/// Host-mediated HTTP capability. Every request goes through the host's
/// domain allow-list, timeout, and size cap.
#[cfg(target_arch = "wasm32")]
pub mod http {
  use serde::de::DeserializeOwned;

  pub use crate::dionysus::metadata::host::{HttpMethod as Method, HttpResponse as Response};

  /// Perform a request exactly as described (method, headers, body).
  pub fn request(
    method: Method,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
  ) -> Result<Response, String> {
    let headers: Vec<(String, String)> =
      headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    crate::dionysus::metadata::host::http_request(method, url, &headers, body)
  }

  /// GET `url` and return status + raw body; non-2xx is *not* an error here
  /// (use [`get_json`] for that).
  pub fn get(url: &str) -> Result<Response, String> {
    request(Method::Get, url, &[], None)
  }

  /// GET `url`, require a 2xx status, and decode the body as JSON.
  pub fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let response = get(url)?;
    if !(200..300).contains(&response.status) {
      return Err(format!("http {} from {url}", response.status));
    }
    serde_json::from_slice(&response.body).map_err(|e| format!("json from {url}: {e}"))
  }
}

/// Emit a message through the host's `tracing` pipeline
/// (see [`log_level`]).
#[cfg(target_arch = "wasm32")]
pub fn log(level: u8, message: &str) {
  crate::dionysus::metadata::host::log(level, message);
}

/// Implement [`Plugin`] and generate the component export boilerplate
/// (MessagePack codecs included). Invoke exactly once, at crate root:
///
/// ```ignore
/// sdk::define_provider!(MyPlugin);
/// ```
#[macro_export]
macro_rules! define_provider {
  ($provider:ty) => {
    const _: () = {
      struct __DionysusMetadataAdapter;

      impl $crate::Guest for __DionysusMetadataAdapter {
        fn descriptor() -> Vec<u8> {
          let descriptor = <$provider as $crate::Plugin>::descriptor();
          match $crate::codec::encode_descriptor(&descriptor) {
            Ok(bytes) => bytes,
            Err(err) => {
              $crate::log($crate::log_level::ERROR, &format!("descriptor encode failed: {err}"));
              Vec::new()
            }
          }
        }

        fn search(query: Vec<u8>) -> Result<Vec<Vec<u8>>, String> {
          let query = $crate::codec::decode_query(&query)?;
          let results = <$provider as $crate::Plugin>::search(&query)?;
          $crate::codec::encode_results(&results)
        }

        fn fetch(id: Vec<u8>) -> Result<Vec<u8>, String> {
          let id = $crate::codec::decode_id(&id)?;
          let metadata = <$provider as $crate::Plugin>::fetch(&id)?;
          $crate::codec::encode_metadata(&metadata)
        }
      }

      $crate::export!(__DionysusMetadataAdapter with_types_in $crate);
    };
  };
}
