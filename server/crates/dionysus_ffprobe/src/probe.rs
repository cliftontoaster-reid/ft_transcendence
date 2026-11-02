//! The probe entry points: run ffprobe, or parse output you already have.

use std::{ffi::OsStr, path::PathBuf};

use crate::{ProbeError, ProbeResult};

/// Parse ffprobe JSON output into a [`ProbeResult`].
pub fn parse(json: &[u8]) -> Result<ProbeResult, ProbeError> {
  Ok(serde_json::from_slice(json)?)
}

/// Run `ffprobe -v error -print_format json -show_streams -show_format` against
/// `path` and parse the result.
pub async fn probe<P: AsRef<OsStr>>(path: P) -> Result<ProbeResult, ProbeError> {
  let path = path.as_ref();
  // Check up front for a friendlier error than ffprobe's opaque exit code.
  let meta = tokio::fs::metadata(path).await.map_err(|e| match e.kind() {
    std::io::ErrorKind::NotFound => ProbeError::NotFound(PathBuf::from(path)),
    _ => ProbeError::Spawn(e),
  })?;
  if !meta.is_file() {
    return Err(ProbeError::NotAFile(PathBuf::from(path)));
  }

  let output = tokio::process::Command::new("ffprobe")
    .args([
      "-v",
      "error",
      "-print_format",
      "json",
      "-show_streams",
      "-show_format",
    ])
    .arg(path)
    .output()
    .await
    .map_err(|e| {
      if e.kind() == std::io::ErrorKind::NotFound {
        ProbeError::MissingExecutable
      } else {
        ProbeError::Spawn(e)
      }
    })?;

  if !output.status.success() {
    return Err(ProbeError::Failed {
      status: output.status,
      stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    });
  }
  parse(&output.stdout)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::StreamInfo;

  #[test]
  fn rejects_invalid_json() {
    assert!(matches!(parse(b"not json"), Err(ProbeError::Parse(_))));
  }

  #[tokio::test]
  async fn probe_missing_file_reports_not_found() {
    let err = probe("/nonexistent/dionysus/probe/test.mkv")
      .await
      .expect_err("missing file must error");
    assert!(matches!(err, ProbeError::NotFound(_)), "got: {err:?}");
  }

  // --- integration: real ffprobe against a generated fixture ---

  /// Generate (once) a 6s 1080p MKV with one video, one audio, and one
  /// English subtitle stream. Requires ffmpeg, like dionysus_ffmpeg's tests.
  fn fixture_path() -> PathBuf {
    let dir = std::env::temp_dir().join("dionysus_ffprobe_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.mkv");
    if path.exists() {
      return path;
    }
    let srt = dir.join("sample.srt");
    std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:02,000\nhello\n\n").unwrap();

    let status = std::process::Command::new("ffmpeg")
      .args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=1920x1080:rate=30:duration=6",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=6",
        "-i",
      ])
      .arg(&srt)
      .args([
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-c:s",
        "srt",
        "-metadata:s:s:0",
        "language=eng",
      ])
      .arg(&path)
      .arg("-y")
      .status()
      .expect("ffmpeg must be installed for tests");
    assert!(status.success(), "fixture generation failed");
    path
  }

  #[tokio::test]
  async fn probes_real_mkv_fixture() {
    let path = fixture_path();
    let probe = probe(&path).await.expect("probe must succeed");

    let video = probe.main_video().expect("video stream");
    assert_eq!(video.codec_name.as_deref(), Some("h264"));
    assert_eq!(video.width, Some(1920));
    assert_eq!(video.height, Some(1080));

    let audio = probe.audio_streams().next().expect("audio stream");
    assert_eq!(audio.codec_name.as_deref(), Some("aac"));
    assert_eq!(audio.sample_rate, Some(44_100));
    assert_eq!(audio.channels, Some(1));

    let subs: Vec<&StreamInfo> = probe.subtitle_streams().collect();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].codec_name.as_deref(), Some("subrip"));
    assert_eq!(subs[0].tags.language.as_deref(), Some("eng"));

    let format = probe.format.as_ref().expect("format");
    assert!(
      format
        .format_name
        .as_deref()
        .is_some_and(|n| n.contains("matroska")),
      "expected matroska, got {:?}",
      format.format_name
    );

    let duration = probe.duration_secs().expect("duration");
    assert!(
      (5.5..6.5).contains(&duration),
      "duration {duration} not near 6s"
    );
  }

  #[tokio::test]
  async fn probe_rejects_unreadable_container() {
    let dir = std::env::temp_dir().join("dionysus_ffprobe_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let junk = dir.join("junk.mkv");
    std::fs::write(&junk, b"this is not a media file").unwrap();

    let err = probe(&junk).await.expect_err("junk file must error");
    assert!(matches!(err, ProbeError::Failed { .. }), "got: {err:?}");
  }
}
