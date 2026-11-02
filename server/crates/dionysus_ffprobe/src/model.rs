//! The typed ffprobe document: containers, streams, dispositions, tags.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::deser::{de_bool_flag, de_opt_f64, de_opt_u32, de_opt_u64};

/// Top-level ffprobe `-show_streams -show_format` document.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ProbeResult {
  #[serde(default)]
  pub streams: Vec<StreamInfo>,
  #[serde(default)]
  pub format: Option<FormatInfo>,
}

/// One entry of ffprobe's `streams` array.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamInfo {
  /// Absolute stream index inside the source container.
  pub index: u32,
  pub codec_type: CodecType,
  #[serde(default)]
  pub codec_name: Option<String>,
  #[serde(default)]
  pub profile: Option<String>,
  #[serde(default)]
  pub width: Option<u32>,
  #[serde(default)]
  pub height: Option<u32>,
  #[serde(default)]
  pub channels: Option<u16>,
  #[serde(default)]
  pub channel_layout: Option<String>,
  #[serde(default, deserialize_with = "de_opt_u32")]
  pub sample_rate: Option<u32>,
  #[serde(default, deserialize_with = "de_opt_u64")]
  pub bit_rate: Option<u64>,
  #[serde(default, deserialize_with = "de_opt_f64")]
  pub duration: Option<f64>,
  #[serde(default)]
  pub disposition: Disposition,
  #[serde(default)]
  pub tags: StreamTags,
}

/// Container-level summary from ffprobe's `format` object.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FormatInfo {
  #[serde(default)]
  pub filename: Option<String>,
  #[serde(default)]
  pub nb_streams: Option<u32>,
  #[serde(default)]
  pub format_name: Option<String>,
  #[serde(default)]
  pub format_long_name: Option<String>,
  #[serde(default, deserialize_with = "de_opt_f64")]
  pub duration: Option<f64>,
  #[serde(default, deserialize_with = "de_opt_u64")]
  pub size: Option<u64>,
  #[serde(default, deserialize_with = "de_opt_u64")]
  pub bit_rate: Option<u64>,
  #[serde(default)]
  pub tags: FormatTags,
}

/// `disposition` flags: ffprobe reports these as 0/1 integers.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Disposition {
  #[serde(default, rename = "default", deserialize_with = "de_bool_flag")]
  pub is_default: bool,
  #[serde(default, deserialize_with = "de_bool_flag")]
  pub forced: bool,
  /// True for cover-art "video" streams, which are not real video tracks.
  #[serde(default, deserialize_with = "de_bool_flag")]
  pub attached_pic: bool,
}

/// `tags` on a stream: language and title are what import cares about.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct StreamTags {
  #[serde(default)]
  pub language: Option<String>,
  #[serde(default)]
  pub title: Option<String>,
}

/// Container-level tags from ffprobe's `format.tags`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FormatTags {
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub language: Option<String>,
}

/// Stream `codec_type`. Unknown values are preserved instead of failing the
/// whole probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecType {
  Video,
  Audio,
  Subtitle,
  Data,
  Attachment,
  Unknown(String),
}

impl CodecType {
  fn as_str(&self) -> &str {
    match self {
      Self::Video => "video",
      Self::Audio => "audio",
      Self::Subtitle => "subtitle",
      Self::Data => "data",
      Self::Attachment => "attachment",
      Self::Unknown(raw) => raw,
    }
  }
}

impl fmt::Display for CodecType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

impl Serialize for CodecType {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}

impl<'de> Deserialize<'de> for CodecType {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let raw = String::deserialize(deserializer)?;
    Ok(match raw.as_str() {
      "video" => Self::Video,
      "audio" => Self::Audio,
      "subtitle" => Self::Subtitle,
      "data" => Self::Data,
      "attachment" => Self::Attachment,
      other => Self::Unknown(other.to_string()),
    })
  }
}

impl ProbeResult {
  /// Video streams that are real tracks (cover art excluded).
  pub fn video_streams(&self) -> impl Iterator<Item = &StreamInfo> {
    self
      .streams
      .iter()
      .filter(|s| s.codec_type == CodecType::Video && !s.disposition.attached_pic)
  }

  pub fn audio_streams(&self) -> impl Iterator<Item = &StreamInfo> {
    self
      .streams
      .iter()
      .filter(|s| s.codec_type == CodecType::Audio)
  }

  pub fn subtitle_streams(&self) -> impl Iterator<Item = &StreamInfo> {
    self
      .streams
      .iter()
      .filter(|s| s.codec_type == CodecType::Subtitle)
  }

  /// The first real (non-cover-art) video stream.
  pub fn main_video(&self) -> Option<&StreamInfo> {
    self.video_streams().next()
  }

  /// First audio stream marked `default`, else the first audio stream.
  pub fn default_audio(&self) -> Option<&StreamInfo> {
    self
      .audio_streams()
      .find(|s| s.disposition.is_default)
      .or_else(|| self.audio_streams().next())
  }

  /// Container duration in seconds, falling back to the longest stream.
  pub fn duration_secs(&self) -> Option<f64> {
    self
      .format
      .as_ref()
      .and_then(|f| f.duration)
      .or_else(|| {
        self
          .streams
          .iter()
          .filter_map(|s| s.duration)
          .reduce(f64::max)
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::parse;

  const TYPICAL: &[u8] = br#"{
  "streams": [
    {
      "index": 0,
      "codec_name": "h264",
      "profile": "High",
      "codec_type": "video",
      "width": 1920,
      "height": 1080,
      "pix_fmt": "yuv420p",
      "bit_rate": "12000000",
      "duration": "6.016000",
      "disposition": { "default": 1, "attached_pic": 0, "forced": 0 },
      "tags": { "language": "und", "title": "Main" }
    },
    {
      "index": 1,
      "codec_name": "aac",
      "codec_type": "audio",
      "sample_rate": "48000",
      "channels": 6,
      "channel_layout": "5.1",
      "bit_rate": "640000",
      "duration": "6.016000",
      "disposition": { "default": 1, "forced": 0 },
      "tags": { "language": "eng", "title": "English 5.1" }
    },
    {
      "index": 2,
      "codec_name": "subrip",
      "codec_type": "subtitle",
      "duration": "5.500000",
      "disposition": { "default": 0, "forced": 1 },
      "tags": { "language": "fre" }
    },
    {
      "index": 3,
      "codec_name": "mjpeg",
      "codec_type": "video",
      "width": 600,
      "height": 882,
      "disposition": { "default": 0, "attached_pic": 1 }
    },
    {
      "index": 4,
      "codec_name": "bin_data",
      "codec_type": "data"
    }
  ],
  "format": {
    "filename": "/library/sample.mkv",
    "nb_streams": 5,
    "format_name": "matroska,webm",
    "format_long_name": "Matroska / WebM",
    "duration": "6.016000",
    "size": "9400000",
    "bit_rate": "12500000"
  }
}"#;

  #[test]
  fn parses_typical_mkv_output() {
    let probe = parse(TYPICAL).expect("typical ffprobe JSON must parse");

    assert_eq!(probe.streams.len(), 5);
    assert_eq!(
      probe.format.as_ref().expect("format").format_name.as_deref(),
      Some("matroska,webm")
    );

    // Stringly numbers parse: duration, size, bit_rate, sample_rate.
    let format = probe.format.as_ref().expect("format");
    assert_eq!(format.duration, Some(6.016));
    assert_eq!(format.size, Some(9_400_000));
    assert_eq!(format.bit_rate, Some(12_500_000));

    // Cover art is excluded from real video streams.
    assert_eq!(probe.video_streams().count(), 1);
    let video = probe.main_video().expect("main video stream");
    assert_eq!(video.index, 0);
    assert_eq!(video.codec_name.as_deref(), Some("h264"));
    assert_eq!(video.width, Some(1920));
    assert_eq!(video.height, Some(1080));
    assert!(video.disposition.is_default);

    let audio = probe.default_audio().expect("audio stream");
    assert_eq!(audio.channels, Some(6));
    assert_eq!(audio.sample_rate, Some(48_000));
    assert_eq!(audio.bit_rate, Some(640_000));
    assert_eq!(audio.tags.language.as_deref(), Some("eng"));
    assert_eq!(audio.tags.title.as_deref(), Some("English 5.1"));

    let subs: Vec<&StreamInfo> = probe.subtitle_streams().collect();
    assert_eq!(subs.len(), 1);
    assert!(subs[0].disposition.forced);
    assert!(!subs[0].disposition.is_default);
    assert_eq!(subs[0].tags.language.as_deref(), Some("fre"));

    assert_eq!(probe.duration_secs(), Some(6.016));
  }

  #[test]
  fn unknown_codec_type_is_preserved() {
    let probe = parse(TYPICAL).unwrap();
    let data = probe
      .streams
      .iter()
      .find(|s| s.index == 4)
      .expect("data stream");
    assert_eq!(data.codec_type, CodecType::Data);

    let probe = parse(br#"{ "streams": [ { "index": 0, "codec_type": "weird" } ] }"#).unwrap();
    assert_eq!(
      probe.streams[0].codec_type,
      CodecType::Unknown("weird".to_string())
    );
    // Round-trips through serde with the raw name.
    let json = serde_json::to_string(&probe.streams[0].codec_type).unwrap();
    assert_eq!(json, "\"weird\"");
  }

  #[test]
  fn tolerates_missing_optional_fields() {
    let probe =
      parse(br#"{ "streams": [ { "index": 0, "codec_type": "audio" } ] }"#).unwrap();
    let stream = &probe.streams[0];
    assert!(stream.codec_name.is_none());
    assert!(stream.channels.is_none());
    assert!(!stream.disposition.is_default);
    assert!(probe.format.is_none());

    // No container duration and no stream durations -> None, not a panic.
    assert_eq!(probe.duration_secs(), None);
  }

  #[test]
  fn duration_falls_back_to_longest_stream() {
    let probe = parse(
      br#"{ "streams": [ { "index": 0, "codec_type": "audio", "duration": "12.5" } ] }"#,
    )
    .unwrap();
    assert_eq!(probe.duration_secs(), Some(12.5));
  }
}
