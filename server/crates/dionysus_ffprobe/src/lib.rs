//! Typed wrapper around `ffprobe -print_format json -show_streams -show_format`.
//!
//! Media import needs more than ffprobe's ad-hoc CSV output: per-stream
//! language/title tags, default/forced/attached_pic dispositions, channel and
//! dimension info, and container duration. [`probe`] shells out to ffprobe and
//! parses its JSON into the types below; [`parse`] does the parsing alone for
//! callers that already have the JSON.
//!
//! Numeric fields (durations, bit rates, sizes, sample rates) are emitted as
//! JSON strings by ffprobe; the deserializers accept both strings and bare
//! numbers and are lenient: an unparseable value becomes `None` rather than an
//! error.

mod deser;
mod error;
mod model;
mod probe;

pub use error::ProbeError;
pub use model::{
  CodecType, Disposition, FormatInfo, FormatTags, ProbeResult, StreamInfo, StreamTags,
};
pub use probe::{parse, probe};
