//! Serde wire types for metadata exchange — shared by native providers, the
//! registry, and (via MessagePack) WASM plugins. Deliberately dependency-free
//! beyond serde: no IO, no Diesel, so wasm32 guests can link it as-is.
//!
//! These types are also the plugin ABI: on the WASM boundary they cross as
//! **MessagePack with named maps** (`rmp_serde::to_vec_named`), never as
//! positional arrays, so field additions stay backward-compatible. Semver on
//! this crate is part of the public plugin contract.

use serde::{Deserialize, Serialize};

/// Version of the model crate, embedded in every `Descriptor` so the host can
/// warn when a plugin was built against a different contract version.
pub const MODEL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
  #[default]
  Movie,
  Episode,
}

/// A lookup for a piece of media — what a provider is asked to search for.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MediaQuery {
  /// Original (pre-split) filename stem, already cleaned by the importer's
  /// parser. Best-effort; `imdb_id`/`tmdb_id` win over it when present.
  pub title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub year: Option<i16>,
  pub kind: MediaKind,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub season: Option<i32>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub episode: Option<i32>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub imdb_id: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub tmdb_id: Option<String>,
}

/// One search hit from a provider. `id` is provider-scoped and is what gets
/// passed back to `fetch`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
  pub id: String,
  pub title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub year: Option<i16>,
  pub kind: MediaKind,
  /// Provider-assigned confidence, 0.0–1.0; results are sorted by it
  /// descending when merged.
  pub score: f32,
}

/// One provider's external id, mirroring the `ExternalPersonID`
/// (`{ provider, value }`) proto convention.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExternalId {
  /// E.g. `"imdb"`, `"tmdb"`.
  pub provider: String,
  pub value: String,
}

/// Per-locale text, mapping to one `media_translations` row. Fields are
/// plain strings; an empty string means "this provider has nothing" so the
/// merge can do first-non-empty-wins per field.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LocalizedText {
  /// BCP-47-ish tag, e.g. `en`, `fr-CA`; `"und"` when unknown.
  pub locale: String,
  pub title: String,
  pub synopsis: String,
  pub summary: String,
}

/// Image kind, mirroring the `media_image_type` DB enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaImageKind {
  Poster,
  Backdrop,
  Logo,
  Banner,
  Still,
  Thumbnail,
}

/// An image to download, mapping to one `media_images` row once fetched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteImage {
  pub url: String,
  pub kind: MediaImageKind,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub locale: Option<String>,
}

/// Role vocabulary mirroring the `credit.proto` `MovieCredit.Role` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CreditRole {
  Actor,
  VoiceActor,
  Cameo,
  StuntPerformer,
  Director,
  AssistantDirector,
  Writer,
  ScreenplayWriter,
  StoryWriter,
  Producer,
  ExecutiveProducer,
  CoProducer,
  LineProducer,
  CastingDirector,
  Cinematographer,
  Editor,
  Colorist,
  ProductionDesigner,
  ArtDirector,
  SetDecorator,
  CostumeDesigner,
  MakeupArtist,
  HairStylist,
  Composer,
  MusicSupervisor,
  SoundDesigner,
  SoundMixer,
  SoundEditor,
  VisualEffectsArtist,
  SpecialEffectsArtist,
  Choreographer,
  Coordinator,
  Narrator,
  ArchiveFootage,
  /// The person appears as themself (`ROLE_SELF` in the proto).
  #[serde(rename = "SELF")]
  SelfRole,
  SpecialThanks,
}

/// One person's involvement, following the `person.proto` / `credit.proto`
/// conventions (`external_ids`, `roles`, `characters`, `billing_order`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credit {
  pub person_external_ids: Vec<ExternalId>,
  pub name: String,
  pub roles: Vec<CreditRole>,
  pub characters: Vec<String>,
  /// Billing order; lower = billed first. 0 when unknown.
  pub billing: i32,
}

/// Everything a provider knows about one piece of media. Merged across
/// providers by the registry.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MediaMetadata {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub release_year: Option<i16>,
  pub kind: MediaKind,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub runtime_secs: Option<i32>,
  pub localized_texts: Vec<LocalizedText>,
  pub images: Vec<RemoteImage>,
  pub credits: Vec<Credit>,
  pub external_ids: Vec<ExternalId>,
}

/// What a provider is (both native and WASM): identity + contract version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Descriptor {
  /// Stable provider id, e.g. `tmdb`, `omdb`, `fanarttv`.
  pub id: String,
  /// Human-readable name.
  pub name: String,
  /// Provider implementation version.
  pub version: String,
  /// `MODEL_VERSION` of the model the provider was built against.
  pub model_version: String,
}

impl Descriptor {
  /// Descriptor stamped with this crate's model version.
  pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
    Self {
      id: id.into(),
      name: name.into(),
      version: version.into(),
      model_version: MODEL_VERSION.to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// The WASM boundary encoding: msgpack with *named* maps. A regression to
  /// positional encoding would silently break every shipped plugin, so pin it
  /// with a test.
  #[test]
  fn msgpack_round_trip_uses_named_maps() {
    let metadata = MediaMetadata {
      release_year: Some(1999),
      kind: MediaKind::Movie,
      runtime_secs: Some(8_100),
      localized_texts: vec![LocalizedText {
        locale: "en".into(),
        title: "The Matrix".into(),
        synopsis: "A hacker learns the truth.".into(),
        summary: "Neo joins the resistance.".into(),
      }],
      images: vec![RemoteImage {
        url: "https://image.tmdb.org/t/p/w500/poster.jpg".into(),
        kind: MediaImageKind::Poster,
        locale: None,
      }],
      credits: vec![Credit {
        person_external_ids: vec![ExternalId {
          provider: "tmdb".into(),
          value: "1".into(),
        }],
        name: "Keanu Reeves".into(),
        roles: vec![CreditRole::Actor],
        characters: vec!["Neo".into()],
        billing: 1,
      }],
      external_ids: vec![
        ExternalId { provider: "imdb".into(), value: "tt0133093".into() },
        ExternalId { provider: "tmdb".into(), value: "603".into() },
      ],
    };

    let bytes = rmp_serde::to_vec_named(&metadata).expect("encode");
    let back: MediaMetadata = rmp_serde::from_slice(&bytes).expect("decode");
    assert_eq!(back, metadata);
  }

  /// Positional encoding would produce an array head (0x90-range with the
  /// struct length); named encoding always starts with a map head (0x80-0x8f
  /// or 0xde). Guard the first byte to make the ABI rule loud.
  #[test]
  fn msgpack_payload_starts_with_a_map() {
    let query = MediaQuery {
      title: "The Matrix".into(),
      year: Some(1999),
      ..Default::default()
    };
    let bytes = rmp_serde::to_vec_named(&query).expect("encode");
    let head = bytes[0];
    assert!(
      matches!(head, 0x80..=0x8f | 0xde),
      "expected msgpack map head, got {head:#x} — use to_vec_named, not to_vec"
    );
  }

  #[test]
  fn unknown_fields_do_not_break_deserialization() {
    // Named maps tolerate a newer model adding fields on an older decoder
    // only if we keep serde defaults; this pins the other direction: extra
    // fields in the payload are ignored by the current model.
    let json = r#"{"title":"Dune","year":2021,"kind":"movie","added_by_future":42}"#;
    let query: MediaQuery = serde_json::from_str(json).expect("decode");
    assert_eq!(query.title, "Dune");
    assert_eq!(query.year, Some(2021));
    assert_eq!(query.kind, MediaKind::Movie);
  }

  #[test]
  fn descriptor_stamps_model_version() {
    assert_eq!(Descriptor::new("tmdb", "TMDB", "0.1.0").model_version, MODEL_VERSION);
  }
}
