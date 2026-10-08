use std::time::Duration;

use dash_mpd::{AdaptationSet, MPD, Period, Representation, SegmentTemplate};
use uuid::Uuid;

use crate::bitrate::{bitrate_for_audio, bitrate_for_video};
use crate::config::AppConfig;
use crate::params::{AudioCodecParam, Quality, VideoCodecParam};

/// 16:9 width for a target height, rounded down to even.
fn width_for_720p_style(height: u32) -> u64 {
  ((height * 16 / 9) & !1) as u64
}

fn qualities() -> [Quality; 3] {
  [Quality::Low, Quality::Medium, Quality::High]
}

/// Build a static VOD manifest advertising the ladder from `config`.
/// Segment URLs are absolute (`/media/{id}/video/...`) so they match
/// the segment routes regardless of where the manifest is served from.
pub fn build_manifest(media_id: &Uuid, config: &AppConfig) -> String {
  let id = media_id.to_string();
  let seg_dur = config.segment_duration_secs;
  let mut adaptations: Vec<AdaptationSet> = Vec::new();
  let mut adapt_idx = 0u32;

  let video_codecs: Vec<VideoCodecParam> = config
    .ladder_video_codecs
    .iter()
    .filter_map(|s| s.parse().ok())
    .collect();
  let video_codecs = if video_codecs.is_empty() {
    vec![VideoCodecParam::H264]
  } else {
    video_codecs
  };
  let resolutions: Vec<u32> = config
    .ladder_resolutions
    .iter()
    .copied()
    .filter(|h| crate::params::KNOWN_HEIGHTS.contains(h))
    .collect();
  let resolutions = if resolutions.is_empty() {
    vec![720]
  } else {
    resolutions
  };

  for codec in &video_codecs {
    for height in &resolutions {
      let reps: Vec<Representation> = qualities()
        .iter()
        .map(|q| Representation {
          id: Some(format!("v-{}-{}-{}", codec.as_str(), height, q.as_str())),
          mimeType: Some("video/mp4".to_string()),
          codecs: Some(codec.mpd_codec().to_string()),
          width: Some(width_for_720p_style(*height)),
          height: Some(*height as u64),
          bandwidth: Some(bitrate_for_video(*q, config) as u64),
          SegmentTemplate: Some(SegmentTemplate {
            media: Some(format!(
              "/media/{id}/video/{height}/{}/{}/$Number$.fmp4",
              codec.as_str(),
              q.as_str()
            )),
            duration: Some(seg_dur as f64),
            timescale: Some(1),
            startNumber: Some(0),
            ..Default::default()
          }),
          ..Default::default()
        })
        .collect();
      adaptations.push(AdaptationSet {
        id: Some(adapt_idx.to_string()),
        contentType: Some("video".to_string()),
        mimeType: Some("video/mp4".to_string()),
        codecs: Some(codec.mpd_codec().to_string()),
        maxWidth: Some(width_for_720p_style(*resolutions.iter().max().unwrap_or(height))),
        maxHeight: Some(*resolutions.iter().max().unwrap_or(height) as u64),
        representations: reps,
        ..Default::default()
      });
      adapt_idx += 1;
    }
  }

  let audio_codecs: Vec<AudioCodecParam> = config
    .ladder_audio_codecs
    .iter()
    .filter_map(|s| s.parse().ok())
    .collect();
  let audio_codecs = if audio_codecs.is_empty() {
    vec![AudioCodecParam::Aac]
  } else {
    audio_codecs
  };
  let channels = if config.ladder_audio_channels.is_empty() {
    vec![2]
  } else {
    config.ladder_audio_channels.clone()
  };

  for codec in &audio_codecs {
    for ch in &channels {
      let reps: Vec<Representation> = qualities()
        .iter()
        .map(|q| Representation {
          id: Some(format!("a-{}-{}ch-{}", codec.as_str(), ch, q.as_str())),
          mimeType: Some("audio/mp4".to_string()),
          codecs: Some(codec.mpd_codec().to_string()),
          bandwidth: Some(bitrate_for_audio(*q, config) as u64),
          audioSamplingRate: Some("48000".to_string()),
          SegmentTemplate: Some(SegmentTemplate {
            media: Some(format!(
              "/media/{id}/audio/{ch}/{}/{}/$Number$.fmp4",
              codec.as_str(),
              q.as_str()
            )),
            duration: Some(seg_dur as f64),
            timescale: Some(1),
            startNumber: Some(0),
            ..Default::default()
          }),
          ..Default::default()
        })
        .collect();
      adaptations.push(AdaptationSet {
        id: Some(adapt_idx.to_string()),
        contentType: Some("audio".to_string()),
        mimeType: Some("audio/mp4".to_string()),
        codecs: Some(codec.mpd_codec().to_string()),
        representations: reps,
        ..Default::default()
      });
      adapt_idx += 1;
    }
  }

  let mpd = MPD {
    mpdtype: Some("static".to_string()),
    xmlns: Some("urn:mpeg:dash:schema:mpd:2011".to_string()),
    profiles: Some("urn:mpeg:dash:profile:isoff-main:2011".to_string()),
    minBufferTime: Some(Duration::from_secs(2)),
    periods: vec![Period {
      id: Some("0".to_string()),
      adaptations,
      ..Default::default()
    }],
    ..Default::default()
  };
  mpd.to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_config() -> AppConfig {
    AppConfig {
      media_root: Default::default(),
      cache_dir: Default::default(),
      cache_max_bytes: 0,
      cache_ttl_secs: 0,
      segment_duration_secs: 4,
      video_bitrate_low: 800_000,
      video_bitrate_medium: 2_500_000,
      video_bitrate_high: 6_000_000,
      audio_bitrate_low: 96_000,
      audio_bitrate_medium: 160_000,
      audio_bitrate_high: 320_000,
      encoder_addr: String::new(),
      ladder_resolutions: vec![480, 720],
      ladder_video_codecs: vec!["h264".to_string()],
      ladder_audio_channels: vec![2],
      ladder_audio_codecs: vec!["aac".to_string()],
    }
  }

  #[test]
  fn manifest_round_trips_and_links_segments() {
    let xml = build_manifest(&Uuid::nil(), &test_config());
    // Links must match the segment routes.
    assert!(xml.contains("/media/00000000-0000-0000-0000-000000000000/video/720/h264/high/$Number$.fmp4"));
    assert!(xml.contains("/media/00000000-0000-0000-0000-000000000000/audio/2/aac/low/$Number$.fmp4"));
    assert!(xml.contains("bandwidth=\"6000000\""));
    // Must parse back as valid MPD.
    let parsed = dash_mpd::parse(&xml).expect("manifest must parse");
    assert_eq!(parsed.periods.len(), 1);
    assert_eq!(parsed.periods[0].adaptations.len(), 3); // 2 video + 1 audio
  }
}
