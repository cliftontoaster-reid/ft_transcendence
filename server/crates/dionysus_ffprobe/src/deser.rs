//! Lenient deserializers for ffprobe's stringly-typed JSON.
//!
//! ffprobe emits most numbers as JSON strings (`"duration": "6.016000"`) and
//! disposition flags as 0/1 integers. These helpers accept both that shape and
//! bare JSON numbers/bools, and prefer `None`/`false` over failing the whole
//! probe on an odd value.

use std::str::FromStr;

use serde::{Deserialize, Deserializer};

/// ffprobe emits most numbers as JSON strings; accept bare numbers too.
#[derive(Deserialize)]
#[serde(untagged)]
enum NumOrStr {
  Num(f64),
  Str(String),
}

impl NumOrStr {
  fn parse_as<T: FromStr>(self) -> Option<T> {
    match self {
      Self::Num(n) => {
        // No lossless generic f64 -> T; callers only use integer T via this
        // path when ffprobe emitted a bare number.
        if n.fract() == 0.0 && n.is_finite() {
          (n as i64).to_string().parse().ok()
        } else {
          None
        }
      }
      Self::Str(s) => s.trim().parse().ok(),
    }
  }
}

pub(super) fn de_opt_f64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
  Ok(
    Option::<NumOrStr>::deserialize(d)?.and_then(|v| match v {
      NumOrStr::Num(n) => Some(n),
      NumOrStr::Str(s) => s.trim().parse().ok(),
    }),
  )
}

pub(super) fn de_opt_u64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
  Ok(Option::<NumOrStr>::deserialize(d)?.and_then(|v| v.parse_as()))
}

pub(super) fn de_opt_u32<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
  Ok(Option::<NumOrStr>::deserialize(d)?.and_then(|v| v.parse_as()))
}

/// Disposition flags arrive as 0/1 integers (some builds as strings/bools).
pub(super) fn de_bool_flag<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
  #[derive(Deserialize)]
  #[serde(untagged)]
  enum Flag {
    Int(u8),
    Str(String),
    Bool(bool),
  }
  Ok(match Flag::deserialize(d)? {
    Flag::Int(n) => n != 0,
    Flag::Str(s) => matches!(s.trim(), "1" | "true" | "yes"),
    Flag::Bool(b) => b,
  })
}
