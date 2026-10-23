//! Media import pipeline: scan a directory, losslessly split each container
//! into per-track files via the encoder's `SplitTracks` gRPC, verify the
//! outputs, and record `media`/`media_track` rows before deleting the
//! original. Metadata enrichment (TMDB/OMDb here, WASM plugins in Phase 6)
//! then resolves ids, fetches provider data, and stores translations,
//! external ids, and images.

pub mod config;
pub mod metadata;
pub mod parser;
pub mod pipeline;
pub mod proto;
pub mod scanner;
