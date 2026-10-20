use rocket::request::FromParam;

use crate::proto::dionysus::encoder::v1::{AudioCodec, Resolution, VideoCodec};

/// `low` | `medium` | `high`, mapped to bitrates from [`crate::config::AppConfig`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
  Low,
  Medium,
  High,
}

impl<'r> FromParam<'r> for Quality {
  type Error = &'r str;

  fn from_param(param: &'r str) -> Result<Self, Self::Error> {
    match param.to_lowercase().as_str() {
      "low" => Ok(Quality::Low),
      "medium" => Ok(Quality::Medium),
      "high" => Ok(Quality::High),
      _ => Err(param),
    }
  }
}

impl Quality {
  pub fn as_str(&self) -> &'static str {
    match self {
      Quality::Low => "low",
      Quality::Medium => "medium",
      Quality::High => "high",
    }
  }
}

/// Target video height in px. Accepts `720` or `720p`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolutionParam(pub u32);

/// Heights we know how to map to the encoder proto.
pub const KNOWN_HEIGHTS: &[u32] = &[240, 360, 480, 720, 1080, 1440, 2160, 4320];

impl<'r> FromParam<'r> for ResolutionParam {
  type Error = &'r str;

  fn from_param(param: &'r str) -> Result<Self, Self::Error> {
    let h: u32 = param
      .trim()
      .trim_end_matches(['p', 'P'])
      .parse()
      .map_err(|_| param)?;
    if KNOWN_HEIGHTS.contains(&h) {
      Ok(ResolutionParam(h))
    } else {
      Err(param)
    }
  }
}

impl ResolutionParam {
  /// Proto enum value for the encoder `VideoJob.rescaling_resolution`.
  pub fn proto_value(&self) -> i32 {
    let r = match self.0 {
      240 => Resolution::Resolution240,
      360 => Resolution::Resolution360,
      480 => Resolution::Resolution480,
      720 => Resolution::Resolution720,
      1080 => Resolution::Resolution1080,
      1440 => Resolution::Resolution1440,
      2160 => Resolution::Resolution2160,
      4320 => Resolution::Resolution4320,
      _ => Resolution::Unspecified,
    };
    r as i32
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodecParam {
  H264,
  H265,
  Vp9,
  Av1,
}

impl<'r> FromParam<'r> for VideoCodecParam {
  type Error = &'r str;

  fn from_param(param: &'r str) -> Result<Self, Self::Error> {
    match param.to_lowercase().as_str() {
      "h264" | "avc" => Ok(VideoCodecParam::H264),
      "h265" | "hevc" => Ok(VideoCodecParam::H265),
      "vp9" => Ok(VideoCodecParam::Vp9),
      "av1" => Ok(VideoCodecParam::Av1),
      _ => Err(param),
    }
  }
}

impl VideoCodecParam {
  pub fn as_str(&self) -> &'static str {
    match self {
      VideoCodecParam::H264 => "h264",
      VideoCodecParam::H265 => "h265",
      VideoCodecParam::Vp9 => "vp9",
      VideoCodecParam::Av1 => "av1",
    }
  }

  pub fn proto_value(&self) -> i32 {
    let c = match self {
      VideoCodecParam::H264 => VideoCodec::H264,
      VideoCodecParam::H265 => VideoCodec::H265,
      VideoCodecParam::Vp9 => VideoCodec::Vp9,
      VideoCodecParam::Av1 => VideoCodec::Av1,
    };
    c as i32
  }

  /// RFC 6381 codec string for the manifest.
  pub fn mpd_codec(&self) -> &'static str {
    match self {
      VideoCodecParam::H264 => "avc1.640028",
      VideoCodecParam::H265 => "hev1.1.6.L120.90",
      VideoCodecParam::Vp9 => "vp09.00.10.08",
      VideoCodecParam::Av1 => "av01.0.05M.08",
    }
  }
}

impl std::str::FromStr for VideoCodecParam {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Self::from_param(s).map_err(|e| e.to_string())
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodecParam {
  Aac,
  Opus,
  Ac3,
  Eac3,
  Mp3,
}

impl<'r> FromParam<'r> for AudioCodecParam {
  type Error = &'r str;

  fn from_param(param: &'r str) -> Result<Self, Self::Error> {
    match param.to_lowercase().as_str() {
      "aac" => Ok(AudioCodecParam::Aac),
      "opus" => Ok(AudioCodecParam::Opus),
      "ac3" => Ok(AudioCodecParam::Ac3),
      "eac3" => Ok(AudioCodecParam::Eac3),
      "mp3" => Ok(AudioCodecParam::Mp3),
      _ => Err(param),
    }
  }
}

impl AudioCodecParam {
  pub fn as_str(&self) -> &'static str {
    match self {
      AudioCodecParam::Aac => "aac",
      AudioCodecParam::Opus => "opus",
      AudioCodecParam::Ac3 => "ac3",
      AudioCodecParam::Eac3 => "eac3",
      AudioCodecParam::Mp3 => "mp3",
    }
  }

  pub fn proto_value(&self) -> i32 {
    let c = match self {
      AudioCodecParam::Aac => AudioCodec::Aac,
      AudioCodecParam::Opus => AudioCodec::Opus,
      AudioCodecParam::Ac3 => AudioCodec::Ac3,
      AudioCodecParam::Eac3 => AudioCodec::Eac3,
      AudioCodecParam::Mp3 => AudioCodec::Mp3,
    };
    c as i32
  }

  pub fn mpd_codec(&self) -> &'static str {
    match self {
      AudioCodecParam::Aac => "mp4a.40.2",
      AudioCodecParam::Opus => "Opus",
      AudioCodecParam::Ac3 => "mp4a.A5",
      AudioCodecParam::Eac3 => "mp4a.A6",
      AudioCodecParam::Mp3 => "mp4a.69",
    }
  }
}

impl std::str::FromStr for AudioCodecParam {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Self::from_param(s).map_err(|e| e.to_string())
  }
}

/// Audio channel count. Accepts `2`/`stereo`, `6`/`5.1`, `8`/`7.1`.
/// Carried in the URL + cache key and advertised in the manifest.
/// (The encoder job currently has no downmix field, so non-native
/// layouts are served as-is until downmix support lands.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelsParam(pub u32);

impl<'r> FromParam<'r> for ChannelsParam {
  type Error = &'r str;

  fn from_param(param: &'r str) -> Result<Self, Self::Error> {
    let normalized = param.trim().to_lowercase();
    let channels = match normalized.as_str() {
      "stereo" => 2,
      "5.1" => 6,
      "7.1" => 8,
      _ => normalized.parse::<u32>().map_err(|_| param)?,
    };
    match channels {
      1 | 2 | 6 | 8 => Ok(ChannelsParam(channels)),
      _ => Err(param),
    }
  }
}

/// Segment filename like `12.fmp4`; rejects anything without the `.fmp4` suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentFile(pub u32);

impl<'r> FromParam<'r> for SegmentFile {
  type Error = &'r str;

  fn from_param(param: &'r str) -> Result<Self, Self::Error> {
    let stem = param.strip_suffix(".fmp4").ok_or(param)?;
    let n: u32 = stem.parse().map_err(|_| param)?;
    Ok(SegmentFile(n))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn quality_parsing() {
    assert_eq!(Quality::from_param("low"), Ok(Quality::Low));
    assert_eq!(Quality::from_param("MEDIUM"), Ok(Quality::Medium));
    assert!(Quality::from_param("ultra").is_err());
  }

  #[test]
  fn resolution_parsing() {
    assert_eq!(
      ResolutionParam::from_param("720"),
      Ok(ResolutionParam(720))
    );
    assert_eq!(
      ResolutionParam::from_param("1080p"),
      Ok(ResolutionParam(1080))
    );
    assert!(ResolutionParam::from_param("999").is_err());
    assert!(ResolutionParam::from_param("abc").is_err());
  }

  #[test]
  fn segment_file_parsing() {
    assert_eq!(SegmentFile::from_param("12.fmp4"), Ok(SegmentFile(12)));
    assert!(SegmentFile::from_param("12.mp4").is_err());
    assert!(SegmentFile::from_param("x.fmp4").is_err());
  }

  #[test]
  fn channels_parsing() {
    assert_eq!(ChannelsParam::from_param("2"), Ok(ChannelsParam(2)));
    assert_eq!(ChannelsParam::from_param("stereo"), Ok(ChannelsParam(2)));
    assert_eq!(ChannelsParam::from_param("5.1"), Ok(ChannelsParam(6)));
    assert!(ChannelsParam::from_param("3").is_err());
  }
}
