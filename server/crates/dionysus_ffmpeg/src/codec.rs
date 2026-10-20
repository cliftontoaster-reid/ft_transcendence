use rust_ffmpeg::{Capabilities, CodecOptions};

use crate::proto::dionysus::encoder::v1::RateControl;
use crate::proto::dionysus::encoder::v1::rate_control::Mode;
use crate::proto::dionysus::encoder::v1::{
  AudioCodec, Filter, HardwareDriver, Resolution, VideoCodec,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
  Amd,
  Intel,
  Nvidia,
  Vaapi,
}

pub fn select_encoder(codec: VideoCodec, platform: Platform, caps: &Capabilities) -> String {
  if codec == VideoCodec::Copy {
    return "copy".to_string();
  }

  let hw_candidates: &[&str] = match (codec, platform) {
    // H.264
    (VideoCodec::H264, Platform::Amd) => &["h264_amf", "h264_vulkan"],
    (VideoCodec::H264, Platform::Intel) => &["h264_qsv", "h264_vulkan"],
    (VideoCodec::H264, Platform::Nvidia) => &["h264_nvenc", "h264_vulkan"],
    (VideoCodec::H264, Platform::Vaapi) => &["h264_vaapi"],

    // HEVC / H.265
    (VideoCodec::H265, Platform::Amd) => &["hevc_amf", "hevc_vulkan"],
    (VideoCodec::H265, Platform::Intel) => &["hevc_qsv", "hevc_vulkan"],
    (VideoCodec::H265, Platform::Nvidia) => &["hevc_nvenc", "hevc_vulkan"],
    (VideoCodec::H265, Platform::Vaapi) => &["hevc_vaapi"],

    // VP9
    (VideoCodec::Vp9, Platform::Intel) => &["vp9_qsv"],
    (VideoCodec::Vp9, Platform::Vaapi) => &["vp9_vaapi"],

    // AV1
    (VideoCodec::Av1, Platform::Amd) => &["av1_amf", "av1_vulkan"],
    (VideoCodec::Av1, Platform::Intel) => &["av1_qsv", "av1_vulkan"],
    (VideoCodec::Av1, Platform::Nvidia) => &["av1_nvenc", "av1_vulkan"],
    (VideoCodec::Av1, Platform::Vaapi) => &["av1_vaapi"],

    // VP8
    (VideoCodec::Vp8, Platform::Intel) => &["vp8_qsv"],
    (VideoCodec::Vp8, Platform::Vaapi) => &["vp8_vaapi"],

    // Motion JPEG
    (VideoCodec::Mjpeg, Platform::Intel) => &["mjpeg_qsv"],
    (VideoCodec::Mjpeg, Platform::Vaapi) => &["mjpeg_vaapi"],

    _ => &[],
  };

  // 1. Check HW candidates against caps.codecs
  for &encoder in hw_candidates {
    if caps.codecs.iter().any(|c| c == encoder) {
      return encoder.to_string();
    }
  }

  // 2. Software fallbacks checked against caps.codecs
  let sw_candidates: &[&str] = match codec {
    VideoCodec::H264 => &["libx264"],
    VideoCodec::H265 => &["libx265"],
    VideoCodec::Vp9 => &["libvpx-vp9"],
    VideoCodec::Av1 => &["libsvtav1", "libaom-av1"],
    VideoCodec::Vp8 => &["libvpx"],
    VideoCodec::Vvc => &["libvvenc"],
    VideoCodec::Mjpeg => &["mjpeg"],
    _ => &[],
  };

  for &encoder in sw_candidates {
    if caps.codecs.iter().any(|c| c == encoder) {
      return encoder.to_string();
    }
  }

  // 3. Fallback string if not found in caps.codecs
  match codec {
    VideoCodec::H264 => "libx264",
    VideoCodec::H265 => "libx265",
    VideoCodec::Vp9 => "libvpx-vp9",
    VideoCodec::Av1 => "libsvtav1",
    VideoCodec::Vp8 => "libvpx",
    VideoCodec::Vvc => "libvvenc",
    VideoCodec::Mjpeg => "mjpeg",
    _ => "copy",
  }
  .to_string()
}

/// Map a job's `HardwareDriver` + free-form `hardware_accelerator` string to a [`Platform`].
pub fn platform_from_job(hardware: Option<i32>, hardware_accelerator: Option<&str>) -> Platform {
  let driver = hardware
    .and_then(|v| HardwareDriver::try_from(v).ok())
    .unwrap_or(HardwareDriver::Unspecified);

  match driver {
    HardwareDriver::Nvenc => Platform::Nvidia,
    HardwareDriver::Amf => Platform::Amd,
    HardwareDriver::Vaapi => Platform::Vaapi,
    HardwareDriver::Vulkan | HardwareDriver::Unspecified => {
      let acc = hardware_accelerator.unwrap_or_default().to_lowercase();
      if acc.contains("nvenc") || acc.contains("nvidia") || acc.contains("cuda") {
        Platform::Nvidia
      } else if acc.contains("amf") || acc.contains("amd") {
        Platform::Amd
      } else if acc.contains("qsv") || acc.contains("intel") {
        Platform::Intel
      } else if acc.contains("vaapi") {
        Platform::Vaapi
      } else if driver == HardwareDriver::Vulkan {
        // Vulkan is cross-vendor; Intel entry tries QSV then Vulkan,
        // so it keeps a HW shot before the software fallback.
        Platform::Intel
      } else {
        Platform::Vaapi
      }
    }
  }
}

/// Pick an audio encoder name for `codec`, consulting `caps` first then falling back.
pub fn select_audio_encoder(codec: AudioCodec, caps: &Capabilities) -> String {
  let candidates: &[&str] = match codec {
    AudioCodec::Aac => &["aac"],
    AudioCodec::Ac3 => &["ac3"],
    AudioCodec::Eac3 => &["eac3"],
    AudioCodec::Opus => &["libopus", "opus"],
    AudioCodec::Vorbis => &["libvorbis", "vorbis"],
    AudioCodec::Ac4 => &["ac4"],
    AudioCodec::Dts => &["dca"],
    AudioCodec::DtsHd => &["dca"],
    AudioCodec::MpegH => &["libmpegh", "aac"],
    AudioCodec::Flac => &["flac"],
    AudioCodec::Pcm => &["pcm_s16le"],
    AudioCodec::Mp3 => &["libmp3lame", "mp3"],
    AudioCodec::Unspecified => &["aac"],
  };

  for &encoder in candidates {
    if caps.codecs.iter().any(|c| c == encoder) {
      return encoder.to_string();
    }
  }

  candidates.first().unwrap_or(&"aac").to_string()
}

/// Target height in pixels for a [`Resolution`]. `None` means "keep source height".
pub fn resolution_height(res: Option<i32>) -> Option<u32> {
  let res = res.and_then(|v| Resolution::try_from(v).ok())?;
  match res {
    Resolution::Unspecified => None,
    Resolution::Resolution240 => Some(240),
    Resolution::Resolution360 => Some(360),
    Resolution::Resolution480 => Some(480),
    Resolution::Resolution720 => Some(720),
    Resolution::Resolution1080 => Some(1080),
    Resolution::Resolution1440 => Some(1440),
    Resolution::Resolution2160 => Some(2160),
    Resolution::Resolution4320 => Some(4320),
  }
}

/// Derive the output width that preserves `src_w/src_h` for `target_h`, rounded even.
pub fn width_for_height(src_w: u32, src_h: u32, target_h: u32) -> u32 {
  if src_w == 0 || src_h == 0 || target_h == 0 {
    return (target_h * 16 / 9) & !1;
  }
  let w = (target_h as f64 * src_w as f64 / src_h as f64).round() as u32;
  (w.max(2) & !1).max(2)
}

/// ffmpeg `scale` filter flag name for a [`Filter`].
pub fn filter_flag(filter: Option<i32>) -> &'static str {
  let f = filter
    .and_then(|v| Filter::try_from(v).ok())
    .unwrap_or(Filter::Unspecified);
  match f {
    Filter::FastBilinear => "fast_bilinear",
    Filter::Bilinear => "bilinear",
    Filter::Bicubic | Filter::Unspecified => "bicubic",
    Filter::Area => "area",
    Filter::Gaussian => "gauss",
    Filter::Sinc => "sinc",
    Filter::Lanczos => "lanczos",
    Filter::Spline => "spline",
  }
}

/// Apply a [`RateControl`] to codec options. No-op when `rc` is `None`/all-zero.
pub fn apply_rate_control(mut opts: CodecOptions, rc: Option<&RateControl>) -> CodecOptions {
  let Some(rc) = rc else {
    return opts;
  };
  if rc.target_bitrate == 0 && rc.min_bitrate == 0 && rc.max_bitrate == 0 && rc.cq == 0 {
    return opts;
  }

  let mode = Mode::try_from(rc.mode).unwrap_or(Mode::Unspecified);
  match mode {
    Mode::Cbr if rc.target_bitrate > 0 => {
      opts = opts
        .bitrate(rc.target_bitrate.to_string())
        .option("minrate", rc.target_bitrate.to_string())
        .option("maxrate", rc.target_bitrate.to_string())
        .option("bufsize", rc.target_bitrate.to_string());
    }
    _ => {
      if rc.target_bitrate > 0 {
        opts = opts.bitrate(rc.target_bitrate.to_string());
      }
      if rc.min_bitrate > 0 {
        opts = opts.option("minrate", rc.min_bitrate.to_string());
      }
      if rc.max_bitrate > 0 {
        opts = opts.option("maxrate", rc.max_bitrate.to_string());
        opts = opts.option("bufsize", (rc.max_bitrate.saturating_mul(2)).to_string());
      }
    }
  }

  if rc.cq > 0 {
    opts = opts.quality(rc.cq.min(u8::MAX as u32) as u8);
  }
  opts
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn driver_maps_to_platform() {
    assert_eq!(
      platform_from_job(Some(HardwareDriver::Nvenc as i32), None),
      Platform::Nvidia
    );
    assert_eq!(
      platform_from_job(Some(HardwareDriver::Amf as i32), None),
      Platform::Amd
    );
    assert_eq!(
      platform_from_job(Some(HardwareDriver::Vaapi as i32), None),
      Platform::Vaapi
    );
    assert_eq!(platform_from_job(None, Some("nvidia")), Platform::Nvidia);
    assert_eq!(platform_from_job(None, Some("amf")), Platform::Amd);
    assert_eq!(platform_from_job(None, None), Platform::Vaapi);
  }

  #[test]
  fn resolution_width_math() {
    assert_eq!(
      resolution_height(Some(Resolution::Resolution720 as i32)),
      Some(720)
    );
    assert_eq!(resolution_height(None), None);
    assert_eq!(
      resolution_height(Some(Resolution::Unspecified as i32)),
      None
    );
    // 1920x1080 -> 720p keeps 16:9
    assert_eq!(width_for_height(1920, 1080, 720), 1280);
    // Odd result rounds to even
    assert_eq!(width_for_height(1920, 1080, 240) % 2, 0);
    // Portrait video
    assert_eq!(width_for_height(1080, 1920, 720), 404);
  }

  #[test]
  fn filter_flags() {
    assert_eq!(filter_flag(Some(Filter::Lanczos as i32)), "lanczos");
    assert_eq!(filter_flag(None), "bicubic");
    assert_eq!(filter_flag(Some(Filter::Area as i32)), "area");
  }
}
