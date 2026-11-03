//! Generated protobuf and gRPC bindings for the Dionysus API.
//!
//! `build.rs` compiles every `.proto` under the repository's `proto/`
//! directory and generates the module tree included below, so new packages
//! are exposed automatically.

include!(concat!(env!("OUT_DIR"), "/proto_tree.rs"));
