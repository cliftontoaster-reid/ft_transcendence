//! MessagePack codecs for the WASM boundary — the *single* implementation
//! used by both sides (the host imports this crate for it too).
//!
//! Rule (plan §boundary encoding): always `rmp_serde::to_vec_named`, never
//! `to_vec`. Named (map) encoding keeps the ABI self-describing so field
//! additions in [`Descriptor`]-bearing model types don't silently corrupt
//! plugins built against an older contract. Errors are plain strings on the
//! WIT boundary, so codec errors surface as `String` as well.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{Descriptor, MediaMetadata, MediaQuery, SearchResult};

/// Encode any boundary payload with named maps.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
  rmp_serde::to_vec_named(value).map_err(|e| format!("msgpack encode: {e}"))
}

/// Decode any boundary payload.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
  rmp_serde::from_slice(bytes).map_err(|e| format!("msgpack decode: {e}"))
}

pub fn encode_query(query: &MediaQuery) -> Result<Vec<u8>, String> {
  encode(query)
}

pub fn decode_query(bytes: &[u8]) -> Result<MediaQuery, String> {
  decode(bytes)
}

/// One MessagePack blob per search hit — the WIT wire shape is
/// `list<list<u8>>`, not one array of hits.
pub fn encode_results(results: &[SearchResult]) -> Result<Vec<Vec<u8>>, String> {
  results.iter().map(encode).collect()
}

pub fn decode_results(wire: &[Vec<u8>]) -> Result<Vec<SearchResult>, String> {
  wire.iter().map(|bytes| decode(bytes)).collect()
}

/// Provider-scoped id crossing as a msgpack string (not raw UTF-8) so the
/// "everything is MessagePack" rule has no exception.
pub fn encode_id(id: &str) -> Result<Vec<u8>, String> {
  encode(&id)
}

pub fn decode_id(bytes: &[u8]) -> Result<String, String> {
  decode(bytes)
}

pub fn encode_metadata(metadata: &MediaMetadata) -> Result<Vec<u8>, String> {
  encode(metadata)
}

pub fn decode_metadata(bytes: &[u8]) -> Result<MediaMetadata, String> {
  decode(bytes)
}

pub fn encode_descriptor(descriptor: &Descriptor) -> Result<Vec<u8>, String> {
  encode(descriptor)
}

pub fn decode_descriptor(bytes: &[u8]) -> Result<Descriptor, String> {
  decode(bytes)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{Credit, CreditRole, ExternalId, LocalizedText, MediaKind, RemoteImage};

  fn query() -> MediaQuery {
    MediaQuery {
      title: "The Matrix".to_string(),
      year: Some(1999),
      ..Default::default()
    }
  }

  #[test]
  fn encode_uses_named_maps() {
    let bytes = encode_query(&query()).expect("encode");
    let head = bytes[0];
    assert!(
      matches!(head, 0x80..=0x8f | 0xde),
      "expected msgpack map head, got {head:#x} — to_vec_named, not to_vec"
    );
  }

  #[test]
  fn query_round_trip() {
    let bytes = encode_query(&query()).expect("encode");
    assert_eq!(decode_query(&bytes).expect("decode"), query());
  }

  #[test]
  fn results_wire_is_one_map_per_hit() {
    let results = vec![
      SearchResult {
        id: "a".to_string(),
        title: "A".to_string(),
        year: None,
        kind: MediaKind::Movie,
        score: 0.9,
      },
      SearchResult {
        id: "b".to_string(),
        title: "B".to_string(),
        year: Some(2020),
        kind: MediaKind::Episode,
        score: 0.4,
      },
    ];
    let wire = encode_results(&results).expect("encode");
    assert_eq!(wire.len(), 2);
    for blob in &wire {
      assert!(matches!(blob[0], 0x80..=0x8f | 0xde), "each hit must be a named map");
    }
    assert_eq!(decode_results(&wire).expect("decode"), results);
  }

  #[test]
  fn id_is_msgpack_string_not_raw_utf8() {
    let bytes = encode_id("tt0133093").expect("encode");
    // A msgpack string head (0xa0..=0xbf short / 0xd9..=0xdb long), never raw bytes.
    assert!(matches!(bytes[0], 0xa0..=0xbf | 0xd9..=0xdb), "got {:#x}", bytes[0]);
    assert_eq!(decode_id(&bytes).expect("decode"), "tt0133093");
  }

  #[test]
  fn metadata_round_trip_covers_every_section() {
    let metadata = MediaMetadata {
      release_year: Some(1999),
      kind: MediaKind::Episode,
      runtime_secs: Some(5_400),
      localized_texts: vec![LocalizedText {
        locale: "en".to_string(),
        title: "Fixture Show".to_string(),
        synopsis: "A fixture synopsis.".to_string(),
        summary: "A fixture summary.".to_string(),
      }],
      images: vec![RemoteImage {
        url: "https://image.tmdb.org/t/p/w500/poster.jpg".to_string(),
        kind: crate::MediaImageKind::Poster,
        locale: None,
      }],
      credits: vec![Credit {
        person_external_ids: vec![ExternalId { provider: "fixture".to_string(), value: "7".to_string() }],
        name: "Ada Example".to_string(),
        roles: vec![CreditRole::Actor],
        characters: vec!["Self".to_string()],
        billing: 0,
      }],
      external_ids: vec![ExternalId { provider: "imdb".to_string(), value: "tt0000001".to_string() }],
    };
    let bytes = encode_metadata(&metadata).expect("encode");
    assert_eq!(decode_metadata(&bytes).expect("decode"), metadata);
  }

  #[test]
  fn descriptor_round_trip() {
    let descriptor = Descriptor::new("fixture", "Fixture Plugin", "0.1.0");
    let bytes = encode_descriptor(&descriptor).expect("encode");
    assert_eq!(decode_descriptor(&bytes).expect("decode"), descriptor);
  }

  #[test]
  fn garbage_decode_is_a_clean_error() {
    let err = decode_query(b"\xff\xff not msgpack").expect_err("must fail");
    assert!(err.starts_with("msgpack decode:"), "{err}");
  }
}
