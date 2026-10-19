//! Wasmtime host-side bindings for the `dionysus:metadata` world.
//!
//! One `bindgen!` for the whole host: imports (our `host` capability
//! interface) are configured `async | trappable`, exports (the plugin's
//! `provider` interface) are called through async wrappers, matching the
//! wasmtime `Engine`/`Store` setup in [`crate::plugin`].
//!
//! The guest side of this same WIT lives in `dionysus_metadata_sdk`
//! (wasm32-only); the two never share generated code, only the WIT file and
//! the MessagePack [`dionysus_metadata_sdk::codec`] payloads.

wasmtime::component::bindgen!({
  world: "metadata-provider",
  path: "../../../proto/wit/dionysus/metadata",
  imports: { default: async | trappable },
  exports: { default: async },
});
