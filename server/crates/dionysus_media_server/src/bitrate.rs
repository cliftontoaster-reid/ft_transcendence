use crate::config::AppConfig;
use crate::params::Quality;
use crate::proto::dionysus::encoder::v1::rate_control::Mode;
use crate::proto::dionysus::encoder::v1::RateControl;

/// `Quality` -> encoder `RateControl`, using per-tier bitrates from env/config.
pub fn video_rate_control(quality: Quality, config: &AppConfig) -> RateControl {
  let target = match quality {
    Quality::Low => config.video_bitrate_low,
    Quality::Medium => config.video_bitrate_medium,
    Quality::High => config.video_bitrate_high,
  };
  vbr(target)
}

pub fn audio_rate_control(quality: Quality, config: &AppConfig) -> RateControl {
  let target = match quality {
    Quality::Low => config.audio_bitrate_low,
    Quality::Medium => config.audio_bitrate_medium,
    Quality::High => config.audio_bitrate_high,
  };
  vbr(target)
}

pub fn bitrate_for_video(quality: Quality, config: &AppConfig) -> u32 {
  match quality {
    Quality::Low => config.video_bitrate_low,
    Quality::Medium => config.video_bitrate_medium,
    Quality::High => config.video_bitrate_high,
  }
}

pub fn bitrate_for_audio(quality: Quality, config: &AppConfig) -> u32 {
  match quality {
    Quality::Low => config.audio_bitrate_low,
    Quality::Medium => config.audio_bitrate_medium,
    Quality::High => config.audio_bitrate_high,
  }
}

fn vbr(target_bitrate: u32) -> RateControl {
  RateControl {
    mode: Mode::Vbr as i32,
    target_bitrate,
    min_bitrate: target_bitrate / 2,
    max_bitrate: target_bitrate.saturating_mul(3) / 2,
    cq: 0,
  }
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
      ladder_resolutions: vec![],
      ladder_video_codecs: vec![],
      ladder_audio_channels: vec![],
      ladder_audio_codecs: vec![],
    }
  }

  #[test]
  fn quality_maps_to_configured_bitrates() {
    let c = test_config();
    assert_eq!(
      video_rate_control(Quality::Low, &c).target_bitrate,
      800_000
    );
    assert_eq!(
      video_rate_control(Quality::High, &c).target_bitrate,
      6_000_000
    );
    assert_eq!(
      audio_rate_control(Quality::Medium, &c).target_bitrate,
      160_000
    );
  }
}
