use std::path::PathBuf;

/// Runtime configuration, all from env with sane defaults.
/// Bitrates are in bits per second.
#[derive(Debug, Clone)]
pub struct AppConfig {
  pub media_root: PathBuf,
  pub cache_dir: PathBuf,
  /// Evict oldest files first when the cache grows past this many bytes.
  pub cache_max_bytes: u64,
  /// Serve cached segments younger than this; older ones are re-transcoded.
  pub cache_ttl_secs: u64,
  /// Media time per segment number increment.
  pub segment_duration_secs: u64,
  pub video_bitrate_low: u32,
  pub video_bitrate_medium: u32,
  pub video_bitrate_high: u32,
  pub audio_bitrate_low: u32,
  pub audio_bitrate_medium: u32,
  pub audio_bitrate_high: u32,
  /// Full gRPC endpoint of the encoder, e.g. `http://[::1]:50051`.
  pub encoder_addr: String,
  /// Rendition ladder advertised in the manifest (heights in px).
  pub ladder_resolutions: Vec<u32>,
  pub ladder_video_codecs: Vec<String>,
  pub ladder_audio_channels: Vec<u32>,
  pub ladder_audio_codecs: Vec<String>,
}

fn env_str(key: &str, default: &str) -> String {
  std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_u32(key: &str, default: u32) -> u32 {
  std::env::var(key)
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
  std::env::var(key)
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(default)
}

fn env_list(key: &str, default: &[&str]) -> Vec<String> {
  std::env::var(key)
    .map(|v| {
      v.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
    })
    .unwrap_or_else(|_| default.iter().map(|s| s.to_string()).collect())
}

fn env_list_u32(key: &str, default: &[u32]) -> Vec<u32> {
  std::env::var(key)
    .map(|v| {
      v.split(',')
        .filter_map(|s| s.trim().trim_end_matches('p').parse().ok())
        .collect()
    })
    .unwrap_or_else(|_| default.to_vec())
}

impl AppConfig {
  pub fn from_env() -> Self {
    Self {
      media_root: PathBuf::from(env_str("MEDIA_ROOT", "./media")),
      cache_dir: PathBuf::from(env_str("CACHE_DIR", "./cache/segments")),
      cache_max_bytes: env_u64("CACHE_MAX_BYTES", 10 * 1024 * 1024 * 1024),
      cache_ttl_secs: env_u64("CACHE_TTL_SECS", 3600),
      segment_duration_secs: env_u64("SEGMENT_DURATION_SECS", 4),
      video_bitrate_low: env_u32("VIDEO_BITRATE_LOW", 800_000),
      video_bitrate_medium: env_u32("VIDEO_BITRATE_MEDIUM", 2_500_000),
      video_bitrate_high: env_u32("VIDEO_BITRATE_HIGH", 6_000_000),
      audio_bitrate_low: env_u32("AUDIO_BITRATE_LOW", 96_000),
      audio_bitrate_medium: env_u32("AUDIO_BITRATE_MEDIUM", 160_000),
      audio_bitrate_high: env_u32("AUDIO_BITRATE_HIGH", 320_000),
      encoder_addr: env_str("ENCODER_ADDR", "http://[::1]:50051"),
      ladder_resolutions: env_list_u32("LADDER_RESOLUTIONS", &[480, 720, 1080]),
      ladder_video_codecs: env_list("LADDER_VIDEO_CODECS", &["h264"]),
      ladder_audio_channels: env_list_u32("LADDER_AUDIO_CHANNELS", &[2, 6]),
      ladder_audio_codecs: env_list("LADDER_AUDIO_CODECS", &["aac"]),
    }
  }
}
