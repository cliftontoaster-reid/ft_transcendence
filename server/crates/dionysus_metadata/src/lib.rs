//! Metadata provider abstraction: the `MetadataProvider` trait, a
//! priority-ordered registry with merge logic, and image downloading.
//! Native providers (Phase 5) and WASM plugins (Phase 6) are
//! indistinguishable here — the WASM host adapter implements this same trait.

pub mod image_fetch;
pub mod registry;

use async_trait::async_trait;

pub use dionysus_metadata_model as model;
pub use image_fetch::{ImageFetcher, ImageFetchError, StoredImage, DEFAULT_IMAGE_DOMAINS};
pub use registry::{merge_metadata, merge_search_results, ProviderRegistry};

use model::{MediaMetadata, MediaQuery, SearchResult};

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
  #[error("not found: {0}")]
  NotFound(String),
  #[error("http {status} from {url}")]
  Http { status: u16, url: String },
  #[error("network: {0}")]
  Network(String),
  #[error("decode: {0}")]
  Decode(String),
  #[error("{0}")]
  Other(String),
}

/// A metadata source. `search` resolves a query to candidate ids (scoped to
/// this provider), `fetch` loads full metadata for one of those ids. The
/// registry attaches the provider id when logging errors, so implementations
/// don't need to repeat it.
#[async_trait]
pub trait MetadataProvider: Send + Sync {
  fn id(&self) -> &str;

  /// Identity + versions this source reports about itself; the default
  /// derives from [`id`](Self::id) — natives override with display names.
  fn descriptor(&self) -> model::Descriptor {
    model::Descriptor::new(self.id(), self.id(), env!("CARGO_PKG_VERSION"))
  }

  async fn search(&self, query: &MediaQuery) -> Result<Vec<SearchResult>, ProviderError>;
  async fn fetch(&self, id: &str) -> Result<MediaMetadata, ProviderError>;
}
