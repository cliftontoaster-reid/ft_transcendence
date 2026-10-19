//! Loading and running `dionysus:metadata` plugins under wasmtime.
//!
//! One [`WasmLoader`] owns the engine/linker; each plugin becomes a
//! [`WasmProvider`], which implements `MetadataProvider` exactly like the
//! native TMDB/OMD b providers — the registry cannot tell the difference.
//! A plugin's store is behind an async mutex: one call at a time per
//! plugin, calls across plugins (and native providers) fully parallel.
//!
//! Failure policy mirrors the rest of the pipeline: a broken or hostile
//! plugin is skipped or degraded to a `ProviderError`, which the importer
//! turns into a scan warning. Nothing a plugin does can fail the host.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dionysus_metadata::{MetadataProvider, ProviderError};
use dionysus_metadata_model::{Descriptor, MediaMetadata, MediaQuery, SearchResult, MODEL_VERSION};
use dionysus_metadata_sdk::codec;
use tokio::sync::Mutex;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

use crate::bindings::MetadataProvider as World;
use crate::host::{id_is_sane, HostState, Trace, DEFAULT_PLUGIN_DOMAINS};
use crate::host::{DEFAULT_MAX_MEMORY_BYTES, DEFAULT_MAX_RESPONSE_BYTES, DEFAULT_TIMEOUT};

#[derive(Debug, thiserror::Error)]
pub enum WasmError {
  #[error("read {path}: {source}")]
  Io { path: PathBuf, source: std::io::Error },
  #[error("wasm engine: {0}")]
  Engine(String),
  #[error("plugin {plugin}: {message}")]
  Plugin { plugin: String, message: String },
  #[error("descriptor: {0}")]
  Descriptor(String),
}

/// What plugins may do and how hard they are capped.
#[derive(Debug, Clone)]
pub struct PluginOptions {
  /// Hosts (`api.tvmaze.com`, …) plugins may HTTP-request; exact or
  /// subdomain-suffix match.
  pub allowed_domains: Vec<String>,
  pub http_timeout: Duration,
  pub max_response_bytes: usize,
  /// Memory cap per plugin instance (`StoreLimits`).
  pub max_memory_bytes: usize,
  /// `METADATA_TRACE_DIR`-style directory receiving pretty JSON dumps of
  /// every payload crossing the boundary.
  pub trace_dir: Option<PathBuf>,
}

impl Default for PluginOptions {
  fn default() -> Self {
    Self {
      allowed_domains: DEFAULT_PLUGIN_DOMAINS.iter().map(|d| d.to_string()).collect(),
      http_timeout: DEFAULT_TIMEOUT,
      max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
      max_memory_bytes: DEFAULT_MAX_MEMORY_BYTES,
      trace_dir: None,
    }
  }
}

impl PluginOptions {
  /// Defaults plus `METADATA_TRACE_DIR` from the environment.
  pub fn from_env() -> Self {
    Self {
      trace_dir: std::env::var_os("METADATA_TRACE_DIR").map(PathBuf::from),
      ..Self::default()
    }
  }

  /// Widen the domain allow-list (plugins' own config arrives in v2;
  /// today this is the programmatic knob).
  pub fn with_allowed_domains<I, S>(mut self, domains: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    self.allowed_domains.extend(domains.into_iter().map(Into::into));
    self
  }

  /// Restrict the allow-list to exactly these hosts.
  pub fn with_only_allowed_domains<I, S>(mut self, domains: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    self.allowed_domains = domains.into_iter().map(Into::into).collect();
    self
  }
}

/// Compiles and instantiates plugin components.
pub struct WasmLoader {
  engine: Engine,
  linker: Linker<HostState>,
  options: Arc<PluginOptions>,
}

impl WasmLoader {
  pub fn new(options: PluginOptions) -> Result<Self, WasmError> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config).map_err(|e| WasmError::Engine(e.to_string()))?;

    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)
      .map_err(|e| WasmError::Engine(format!("wasi: {e}")))?;
    World::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |state| state)
      .map_err(|e| WasmError::Engine(format!("host imports: {e}")))?;

    Ok(Self { engine, linker, options: Arc::new(options) })
  }

  /// Loader configured from the environment ([`PluginOptions::from_env`]).
  pub fn from_env() -> Result<Self, WasmError> {
    Self::new(PluginOptions::from_env())
  }

  pub fn options(&self) -> &PluginOptions {
    &self.options
  }

  /// Every `*.wasm` in `dir`, sorted by path. Broken files are reported in
  /// the error list, never abort the batch.
  pub fn load_dir(&self, dir: &Path) -> (Vec<WasmProvider>, Vec<WasmError>) {
    let mut providers = Vec::new();
    let mut errors = Vec::new();

    let entries = match std::fs::read_dir(dir) {
      Ok(entries) => entries,
      Err(source) => {
        errors.push(WasmError::Io { path: dir.to_path_buf(), source });
        return (providers, errors);
      }
    };
    let mut paths: Vec<PathBuf> = entries
      .filter_map(|entry| entry.ok())
      .map(|entry| entry.path())
      .filter(|path| path.extension().is_some_and(|ext| ext == "wasm"))
      .collect();
    paths.sort();

    let mut seen: Vec<String> = Vec::new();
    for path in paths {
      let name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "plugin".to_string());
      match self.load_file(&path) {
        Ok(provider) => {
          if seen.contains(&provider.descriptor().id) {
            errors.push(WasmError::Plugin {
              plugin: name,
              message: format!("duplicate provider id {}", provider.descriptor().id),
            });
          } else {
            seen.push(provider.descriptor().id.clone());
            providers.push(provider);
          }
        }
        Err(err) => errors.push(err),
      }
    }
    (providers, errors)
  }

  pub fn load_file(&self, path: &Path) -> Result<WasmProvider, WasmError> {
    let name =
      path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "plugin".into());
    let bytes = std::fs::read(path).map_err(|source| WasmError::Io { path: path.to_path_buf(), source })?;
    self.load_bytes(&name, &bytes)
  }

  /// Load one component. Instantiation and the initial `descriptor()` call
  /// run on a throwaway executor — they touch no IO (a `descriptor()`
  /// that does HTTP is a plugin bug and will fail loudly).
  pub fn load_bytes(&self, name: &str, bytes: &[u8]) -> Result<WasmProvider, WasmError> {
    let component = Component::new(&self.engine, bytes).map_err(|e| WasmError::Plugin {
      plugin: name.to_string(),
      message: format!("compile: {e}"),
    })?;

    let trace = Arc::new(Trace::new(self.options.trace_dir.clone()));
    let state = HostState::new(self.options.clone(), name.to_string());
    let mut store = Store::new(&self.engine, state);
    store.limiter(|state| state.limiter());

    let (descriptor_bytes, instance) = futures::executor::block_on(async {
      let instance = World::instantiate_async(&mut store, &component, &self.linker)
        .await
        .map_err(|e| WasmError::Plugin {
          plugin: name.to_string(),
          message: format!("instantiate: {e}"),
        })?;
      let descriptor_bytes = instance
        .dionysus_metadata_provider()
        .call_descriptor(&mut store)
        .await
        .map_err(|e| WasmError::Plugin {
          plugin: name.to_string(),
          message: format!("descriptor trap: {e}"),
        })?;
      Ok::<_, WasmError>((descriptor_bytes, instance))
    })?;

    let descriptor = codec::decode_descriptor(&descriptor_bytes)
      .map_err(|e| WasmError::Descriptor(format!("{name}: {e}")))?;
    if !id_is_sane(&descriptor.id) {
      return Err(WasmError::Descriptor(format!(
        "{name}: unusable provider id {:?}",
        descriptor.id
      )));
    }
    if descriptor.model_version != MODEL_VERSION {
      tracing::warn!(
        plugin = %descriptor.id,
        plugin_model = %descriptor.model_version,
        host_model = MODEL_VERSION,
        "plugin built against a different model version; payloads may not decode"
      );
    }
    trace.dump(&descriptor.id, "descriptor", "out", &descriptor);

    Ok(WasmProvider {
      id: descriptor.id.clone(),
      descriptor,
      runtime: Mutex::new(Runtime { store, instance }),
      trace,
    })
  }
}

struct Runtime {
  store: Store<HostState>,
  instance: World,
}

/// One loaded plugin, indistinguishable from a native provider.
pub struct WasmProvider {
  id: String,
  descriptor: Descriptor,
  runtime: Mutex<Runtime>,
  trace: Arc<Trace>,
}

impl WasmProvider {
  pub fn descriptor(&self) -> &Descriptor {
    &self.descriptor
  }
}

impl std::fmt::Debug for WasmProvider {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // The store is deliberately omitted — it is runtime state, not identity.
    f.debug_struct("WasmProvider").field("descriptor", &self.descriptor).finish()
  }
}

#[async_trait]
impl MetadataProvider for WasmProvider {
  fn id(&self) -> &str {
    &self.id
  }

  fn descriptor(&self) -> Descriptor {
    self.descriptor.clone()
  }

  async fn search(&self, query: &MediaQuery) -> Result<Vec<SearchResult>, ProviderError> {
    let wire = codec::encode_query(query).map_err(ProviderError::Decode)?;
    self.trace.dump(&self.id, "search", "in", query);

    let mut runtime = self.runtime.lock().await;
    let Runtime { store, instance } = &mut *runtime;
    let hits = instance
      .dionysus_metadata_provider()
      .call_search(store, &wire)
      .await
      .map_err(|e| ProviderError::Other(format!("plugin {} trapped: {e}", self.id)))?
      .map_err(|e| ProviderError::Other(format!("plugin {}: {e}", self.id)))?;

    let results = codec::decode_results(&hits).map_err(ProviderError::Decode)?;
    self.trace.dump(&self.id, "search", "out", &results);
    Ok(results)
  }

  async fn fetch(&self, id: &str) -> Result<MediaMetadata, ProviderError> {
    let wire = codec::encode_id(id).map_err(ProviderError::Decode)?;
    self.trace.dump(&self.id, "fetch", "in", &id.to_string());

    let mut runtime = self.runtime.lock().await;
    let Runtime { store, instance } = &mut *runtime;
    let metadata = instance
      .dionysus_metadata_provider()
      .call_fetch(store, &wire)
      .await
      .map_err(|e| ProviderError::Other(format!("plugin {} trapped: {e}", self.id)))?
      .map_err(|e| ProviderError::Other(format!("plugin {}: {e}", self.id)))?;

    let metadata = codec::decode_metadata(&metadata).map_err(ProviderError::Decode)?;
    self.trace.dump(&self.id, "fetch", "out", &metadata);
    Ok(metadata)
  }
}

/// Load every `PLUGINS_DIR/*.wasm`. `PLUGINS_DIR` unset → no plugins.
/// Individual broken plugins are warnings; only a broken engine is an error.
pub fn load_plugins_from_env() -> Result<Vec<WasmProvider>, WasmError> {
  let Some(dir) = std::env::var_os("PLUGINS_DIR") else { return Ok(Vec::new()) };
  let loader = WasmLoader::from_env()?;
  let (providers, errors) = loader.load_dir(Path::new(&dir));
  for err in errors {
    tracing::warn!("plugin skipped: {err}");
  }
  tracing::info!(dir = %Path::new(&dir).display(), count = providers.len(), "loaded wasm plugins");
  Ok(providers)
}
