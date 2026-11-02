//! Host runtime for `dionysus:metadata` WASM plugins.
//!
//! Plugins are components (built for `wasm32-wasip2` with
//! `dionysus_metadata_sdk`) that the host compiles with wasmtime and runs
//! behind a capability API: mediated HTTP (domain allow-list, timeout,
//! size cap, memory limit) and a `log` bridge into `tracing`. A plugin
//! implements `MetadataProvider` at runtime, so [`WasmProvider`] slots into
//! the same `ProviderRegistry` as the native providers — the registry
//! cannot tell the difference.
//!
//! Configuration is two environment variables, both handled at load:
//!
//! - `PLUGINS_DIR` — directory of `*.wasm` files; each becomes a
//!   low-priority provider. Broken plugins are warnings.
//! - `METADATA_TRACE_DIR` — receives pretty JSON dumps of every payload
//!   crossing the boundary (debugging aid).
//!
//! WASI is a blank context: no filesystem, no env, no stdio. The only
//! capabilities a plugin has are the ones granted through the WIT `host`
//! interface.

mod bindings;
mod host;
mod plugin;

pub use host::DEFAULT_PLUGIN_DOMAINS;
pub use plugin::{
  load_plugins_from_env, PluginOptions, WasmError, WasmLoader, WasmProvider,
};
