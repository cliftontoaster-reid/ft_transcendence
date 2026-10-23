//! `SplitTracks`: lossless demux of one source container into one file per
//! track (`.mkv` video, `.mka` audio, `.mks` subtitles).
//!
//! One ffmpeg run per track with stream copy — a failing run only affects its
//! own track, and nothing is ever re-encoded. The caller (the importer)
//! verifies the produced files; this module only guarantees the runs succeeded
//! and describes what it wrote.

use std::path::{Path, PathBuf};

use dionysus_ffprobe::{CodecType, ProbeError, ProbeResult, StreamInfo};
use rust_ffmpeg::{Codec, FFmpegBuilder, Input, LogLevel, Output, StreamMap};
use tonic::Status;

use crate::jobs::JobServer;
use crate::proto::dionysus::encoder::v1::{
  MediaTrackType, SplitTrack, SplitTracksRequest, SplitTracksResponse,
};

/// Subtitle codecs Matroska cannot carry: attempt a text conversion to SRT
/// instead of a stream copy; if the conversion fails the track is skipped
/// with a warning (subtitles are best-effort).
const MKV_UNSUPPORTED_SUB_CODECS: &[&str] = &["dvb_teletext", "mov_text", "eia_608", "eia_708"];

/// One track selected for splitting out of a source container.
struct PlannedTrack<'a> {
  stream: &'a StreamInfo,
  track_type: MediaTrackType,
  /// Per-type ordinal naming the output file (`video-0.mkv`, `audio-1.mka`, ...).
  ordinal: usize,
  /// Subtitle codec Matroska cannot carry: convert to SRT instead of copying.
  convert_subtitle: bool,
}

impl PlannedTrack<'_> {
  fn type_name(&self) -> &'static str {
    match self.track_type {
      MediaTrackType::Video => "video",
      MediaTrackType::Audio => "audio",
      MediaTrackType::Subtitles => "subtitles",
      MediaTrackType::Unspecified => unreachable!("plan() never yields unspecified tracks"),
    }
  }

  fn filename(&self) -> String {
    let ext = match self.track_type {
      MediaTrackType::Video => "mkv",
      MediaTrackType::Audio => "mka",
      MediaTrackType::Subtitles => "mks",
      MediaTrackType::Unspecified => unreachable!("plan() never yields unspecified tracks"),
    };
    format!("{}-{}.{}", self.type_name(), self.ordinal, ext)
  }
}

/// Pick the streams worth splitting: real video tracks (cover art excluded),
/// audio, and subtitles. Data, attachment, and unknown streams are not track
/// files. Returns the plan plus informational warnings.
fn plan(probe: &ProbeResult) -> (Vec<PlannedTrack<'_>>, Vec<String>) {
  let mut tracks = Vec::new();
  let mut warnings = Vec::new();
  let mut ordinals = [0usize; 3]; // video, audio, subtitles

  for stream in &probe.streams {
    let slot = match stream.codec_type {
      CodecType::Video if stream.disposition.attached_pic => continue, // cover art
      CodecType::Video => 0,
      CodecType::Audio => 1,
      CodecType::Subtitle => 2,
      // Data, attachment, and unknown streams are not split into track files.
      _ => continue,
    };
    let track_type = match slot {
      0 => MediaTrackType::Video,
      1 => MediaTrackType::Audio,
      _ => MediaTrackType::Subtitles,
    };

    let convert_subtitle = track_type == MediaTrackType::Subtitles
      && stream
        .codec_name
        .as_deref()
        .is_some_and(|c| MKV_UNSUPPORTED_SUB_CODECS.contains(&c));
    if convert_subtitle {
      warnings.push(format!(
        "stream {}: subtitle codec {} is not Matroska-compatible; converting to srt",
        stream.index,
        stream.codec_name.as_deref().unwrap_or("unknown")
      ));
    }

    let ordinal = ordinals[slot];
    ordinals[slot] += 1;
    tracks.push(PlannedTrack {
      stream,
      track_type,
      ordinal,
      convert_subtitle,
    });
  }
  (tracks, warnings)
}

impl JobServer {
  /// Probe `request.source_path`, split every real track into its own file
  /// under `request.output_dir`, and describe the results.
  pub(crate) async fn split(
    &self,
    request: SplitTracksRequest,
  ) -> Result<SplitTracksResponse, Status> {
    if request.source_path.trim().is_empty() {
      return Err(Status::invalid_argument(
        "SplitTracksRequest.source_path is empty",
      ));
    }
    if request.output_dir.trim().is_empty() {
      return Err(Status::invalid_argument(
        "SplitTracksRequest.output_dir is empty",
      ));
    }

    let probe = dionysus_ffprobe::probe(&request.source_path)
      .await
      .map_err(map_probe_error)?;

    tokio::fs::create_dir_all(&request.output_dir)
      .await
      .map_err(|e| {
        Status::internal(format!(
          "cannot create output dir {}: {e}",
          request.output_dir
        ))
      })?;

    let (planned, mut warnings) = plan(&probe);
    let mut response_tracks = Vec::with_capacity(planned.len());

    for track in planned {
      let out_path = PathBuf::from(&request.output_dir).join(track.filename());
      if let Err(err) = split_track(&request.source_path, &track, &out_path).await {
        // Never leave a partial file behind.
        tokio::fs::remove_file(&out_path).await.ok();
        if track.track_type == MediaTrackType::Subtitles {
          // Subtitles are best-effort: an odd codec must not fail the import.
          warnings.push(format!(
            "stream {}: subtitle split failed, skipping: {err}",
            track.stream.index
          ));
          continue;
        }
        return Err(Status::internal(format!(
          "failed to split stream {} ({}): {err}",
          track.stream.index,
          track.stream.codec_name.as_deref().unwrap_or("unknown")
        )));
      }
      response_tracks.push(describe(&track, &out_path));
    }

    Ok(SplitTracksResponse {
      tracks: response_tracks,
      warnings,
    })
  }
}

/// One ffmpeg run: `-map 0:{N} -c copy` into a per-track Matroska file.
async fn split_track(
  source: &str,
  track: &PlannedTrack<'_>,
  out_path: &Path,
) -> Result<(), String> {
  let mut builder = FFmpegBuilder::new()
    .map_err(|e| format!("ffmpeg not available: {e}"))?
    .log_level(LogLevel::Error)
    .input(Input::new(source.to_string()))
    .map(StreamMap::stream_index(0, track.stream.index as usize))
    .overwrite();

  let mut output = Output::new(out_path.to_string_lossy().into_owned())
    .format("matroska") // .mkv / .mka / .mks are all Matroska
    .copy_codecs();

  if track.track_type == MediaTrackType::Subtitles {
    // Font attachments referenced by ASS/SSA tracks follow the subtitle
    // outputs; the trailing '?' tolerates sources without attachments. The
    // raw arg lands after the track map, so the track stays output stream 0.
    builder = builder.raw_args(["-map", "0:t?"]);
    output = output.option("c:t", "copy");
  }

  if track.convert_subtitle {
    output = output.subtitle_codec(Codec::new("srt"));
  }

  let stream = track.stream;
  output = output.stream_metadata(
    "0",
    "language",
    stream.tags.language.as_deref().unwrap_or("und"),
  );
  if let Some(title) = stream.tags.title.as_deref() {
    output = output.stream_metadata("0", "title", title);
  }
  // Re-assert default/forced explicitly: stream copy must not let the muxer
  // invent or drop dispositions.
  output = output.option("disposition:0", disposition_flags(stream));

  builder
    .output(output)
    .run()
    .await
    .map(drop)
    .map_err(|e| e.to_string())
}

/// `-disposition:0` value mirroring the source track's default/forced flags.
fn disposition_flags(stream: &StreamInfo) -> String {
  let d = &stream.disposition;
  let mut flags = Vec::new();
  if d.is_default {
    flags.push("default");
  }
  if d.forced {
    flags.push("forced");
  }
  if flags.is_empty() {
    "0".to_string()
  } else {
    flags.join("+")
  }
}

/// Describe one produced file from the plan and the source stream.
fn describe(track: &PlannedTrack<'_>, out_path: &Path) -> SplitTrack {
  let stream = track.stream;
  SplitTrack {
    track_type: track.track_type as i32,
    stream_index: stream.index as i32,
    path: out_path.to_string_lossy().into_owned(),
    codec: if track.convert_subtitle {
      "srt".to_string()
    } else {
      stream.codec_name.clone().unwrap_or_default()
    },
    language: stream
      .tags
      .language
      .clone()
      .unwrap_or_else(|| "und".to_string()),
    title: stream.tags.title.clone().unwrap_or_default(),
    is_default: stream.disposition.is_default,
    is_forced: stream.disposition.forced,
    channels: stream.channels.unwrap_or(0) as i32,
    width: stream.width.unwrap_or(0) as i32,
    height: stream.height.unwrap_or(0) as i32,
  }
}

fn map_probe_error(err: ProbeError) -> Status {
  match err {
    ProbeError::NotFound(path) | ProbeError::NotAFile(path) => {
      Status::not_found(format!("media file: {}", path.display()))
    }
    ProbeError::MissingExecutable => {
      Status::failed_precondition("ffprobe executable not found; install ffmpeg")
    }
    ProbeError::Failed { stderr, .. } => Status::internal(format!("ffprobe failed: {stderr}")),
    ProbeError::Spawn(e) => Status::internal(format!("failed to run ffprobe: {e}")),
    ProbeError::Parse(e) => Status::internal(format!("cannot parse ffprobe output: {e}")),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proto::dionysus::encoder::v1::SplitTracksRequest;

  fn request(source: &Path, output_dir: &Path) -> SplitTracksRequest {
    SplitTracksRequest {
      id: None,
      source_path: source.to_string_lossy().into_owned(),
      output_dir: output_dir.to_string_lossy().into_owned(),
    }
  }

  // --- plan(): stream selection without touching ffmpeg ---

  const SELECTABLE: &[u8] = br#"{
  "streams": [
    { "index": 0, "codec_name": "h264", "codec_type": "video",
      "disposition": { "default": 1 } },
    { "index": 1, "codec_name": "aac", "codec_type": "audio",
      "disposition": { "default": 1 } },
    { "index": 2, "codec_name": "mov_text", "codec_type": "subtitle",
      "tags": { "language": "eng" },
      "disposition": { "default": 0, "forced": 1 } },
    { "index": 3, "codec_name": "mjpeg", "codec_type": "video",
      "disposition": { "attached_pic": 1 } },
    { "index": 4, "codec_name": "bin_data", "codec_type": "data" }
  ]
}"#;

  #[test]
  fn plan_selects_real_tracks_and_skips_cover_art_and_data() {
    let probe = dionysus_ffprobe::parse(SELECTABLE).unwrap();
    let (tracks, warnings) = plan(&probe);

    let filenames: Vec<String> = tracks.iter().map(PlannedTrack::filename).collect();
    // Cover art (stream 3) and the data stream (stream 4) are not tracks;
    // the mov_text subtitle still gets a slot, flagged for conversion.
    assert_eq!(filenames, ["video-0.mkv", "audio-0.mka", "subtitles-0.mks"]);
    assert_eq!(tracks[2].stream.index, 2);
    assert!(tracks[2].convert_subtitle);

    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("mov_text"), "got: {}", warnings[0]);
    assert!(warnings[0].contains("converting to srt"));
  }

  #[test]
  fn plan_numbers_tracks_per_type_from_zero() {
    let probe = dionysus_ffprobe::parse(
      br#"{ "streams": [
        { "index": 0, "codec_type": "audio" },
        { "index": 1, "codec_type": "video", "disposition": { "attached_pic": 1 } },
        { "index": 2, "codec_type": "audio" },
        { "index": 3, "codec_type": "video" }
      ] }"#,
    )
    .unwrap();
    let (tracks, warnings) = plan(&probe);
    let filenames: Vec<String> = tracks.iter().map(PlannedTrack::filename).collect();
    assert_eq!(filenames, ["audio-0.mka", "audio-1.mka", "video-0.mkv"]);
    assert!(warnings.is_empty());
  }

  #[test]
  fn disposition_flags_mirror_source() {
    let probe = dionysus_ffprobe::parse(
      br#"{ "streams": [
        { "index": 0, "codec_type": "audio",
          "disposition": { "default": 1, "forced": 1 } },
        { "index": 1, "codec_type": "audio",
          "disposition": { "default": 0, "forced": 0 } }
      ] }"#,
    )
    .unwrap();
    assert_eq!(disposition_flags(&probe.streams[0]), "default+forced");
    assert_eq!(disposition_flags(&probe.streams[1]), "0");
  }

  // --- integration: real ffmpeg runs against generated fixtures ---

  /// Muxed fixture: h264 video, eng AAC audio (default), spa AAC audio,
  /// eng subrip (default), fre subrip (forced), plus one dummy attachment.
  fn muxed_fixture_path() -> PathBuf {
    let dir = std::env::temp_dir().join("dionysus_ffmpeg_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("split_muxed.mkv");
    if path.exists() {
      return path;
    }
    let sub_eng = dir.join("split_eng.srt");
    let sub_fre = dir.join("split_fre.srt");
    std::fs::write(&sub_eng, "1\n00:00:00,000 --> 00:00:02,000\nhello\n\n").unwrap();
    std::fs::write(&sub_fre, "1\n00:00:00,000 --> 00:00:02,000\nbonjour\n\n").unwrap();

    let status = std::process::Command::new("ffmpeg")
      .args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=640x360:rate=30:duration=6",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=6",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=880:duration=6",
        "-i",
      ])
      .arg(&sub_eng)
      .arg("-i")
      .arg(&sub_fre)
      .args([
        "-map",
        "0:v:0",
        "-map",
        "1:a:0",
        "-map",
        "2:a:0",
        "-map",
        "3:s:0",
        "-map",
        "4:s:0",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-c:s",
        "srt",
        "-metadata:s:a:0",
        "language=eng",
        "-metadata:s:a:1",
        "language=spa",
        "-metadata:s:s:0",
        "language=eng",
        "-metadata:s:s:1",
        "language=fre",
        "-disposition:a:0",
        "default",
        "-disposition:a:1",
        "0",
        "-disposition:s:0",
        "default",
        "-disposition:s:1",
        "forced",
        "-attach",
      ])
      .arg(&sub_eng)
      .args(["-metadata:s:t:0", "mimetype=text/plain"])
      .args(["-metadata:s:t:0", "filename=eng.srt"])
      .arg(&path)
      .arg("-y")
      .status()
      .expect("ffmpeg must be installed for tests");
    assert!(status.success(), "fixture generation failed");
    path
  }

  /// Fixture with an MP4-only subtitle codec (mov_text), which Matroska
  /// cannot carry: the split must convert it to SRT and warn.
  fn movtext_fixture_path() -> PathBuf {
    let dir = std::env::temp_dir().join("dionysus_ffmpeg_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("split_movtext.mp4");
    if path.exists() {
      return path;
    }
    let sub = dir.join("split_movtext.srt");
    std::fs::write(&sub, "1\n00:00:00,000 --> 00:00:02,000\ntx3g\n\n").unwrap();

    let status = std::process::Command::new("ffmpeg")
      .args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=640x360:rate=30:duration=6",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=6",
        "-i",
      ])
      .arg(&sub)
      .args([
        "-map",
        "0:v:0",
        "-map",
        "1:a:0",
        "-map",
        "2:s:0",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-c:s",
        "mov_text",
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
  async fn split_tracks_demuxes_every_track_losslessly() {
    let source = muxed_fixture_path();
    let out = tempfile::tempdir().expect("temp output dir");
    let server = JobServer::new(1).await;

    let response = server
      .split(request(&source, out.path()))
      .await
      .expect("split must succeed");
    assert!(response.warnings.is_empty(), "warnings: {response:?}");

    // One file per real track, named by per-type ordinal.
    let names: Vec<String> = response
      .tracks
      .iter()
      .map(|t| {
        Path::new(&t.path)
          .file_name()
          .unwrap()
          .to_string_lossy()
          .into_owned()
      })
      .collect();
    assert_eq!(
      names,
      [
        "video-0.mkv",
        "audio-0.mka",
        "audio-1.mka",
        "subtitles-0.mks",
        "subtitles-1.mks"
      ]
    );
    for track in &response.tracks {
      assert!(Path::new(&track.path).is_file(), "missing {}", track.path);
    }

    // Video: exactly one stream, source dimensions and codec intact.
    let video = dionysus_ffprobe::probe(&response.tracks[0].path)
      .await
      .unwrap();
    assert_eq!(video.streams.len(), 1, "video file must hold one stream");
    let v = video.main_video().expect("video stream");
    assert_eq!(v.codec_name.as_deref(), Some("h264"));
    assert_eq!((v.width, v.height), (Some(640), Some(360)));
    assert_eq!(response.tracks[0].width, 640);
    assert_eq!(response.tracks[0].height, 360);
    let duration = video.duration_secs().expect("duration");
    assert!((5.5..6.5).contains(&duration), "duration {duration}");

    // Audio: language tags and default disposition preserved per file.
    let audio0 = dionysus_ffprobe::probe(&response.tracks[1].path)
      .await
      .unwrap();
    assert_eq!(audio0.streams.len(), 1);
    let a0 = audio0.audio_streams().next().expect("audio stream");
    assert_eq!(a0.codec_name.as_deref(), Some("aac"));
    assert_eq!(a0.tags.language.as_deref(), Some("eng"));
    assert!(a0.disposition.is_default, "first audio stays default");
    assert!(response.tracks[1].is_default);

    let audio1 = dionysus_ffprobe::probe(&response.tracks[2].path)
      .await
      .unwrap();
    assert_eq!(audio1.streams.len(), 1);
    let a1 = audio1.audio_streams().next().expect("audio stream");
    assert_eq!(a1.tags.language.as_deref(), Some("spa"));
    assert!(
      !a1.disposition.is_default,
      "second audio must not be default in its own file"
    );
    assert!(!response.tracks[2].is_default);
    assert_eq!(response.tracks[2].channels, a1.channels.unwrap() as i32);

    // Subtitles: language, default/forced flags, and the source attachment
    // (font files for ASS) ride along with the subtitle outputs.
    let subs0 = dionysus_ffprobe::probe(&response.tracks[3].path)
      .await
      .unwrap();
    assert_eq!(subs0.streams.len(), 2, "subtitle + attachment");
    let s0 = subs0.subtitle_streams().next().expect("subtitle stream");
    assert_eq!(s0.codec_name.as_deref(), Some("subrip"));
    assert_eq!(s0.tags.language.as_deref(), Some("eng"));
    assert!(s0.disposition.is_default);
    assert!(
      subs0
        .streams
        .iter()
        .any(|s| s.codec_type == CodecType::Attachment),
      "attachment must follow the subtitles"
    );

    let subs1 = dionysus_ffprobe::probe(&response.tracks[4].path)
      .await
      .unwrap();
    let s1 = subs1.subtitle_streams().next().expect("subtitle stream");
    assert_eq!(s1.tags.language.as_deref(), Some("fre"));
    assert!(s1.disposition.forced);
    assert!(response.tracks[4].is_forced);
  }

  #[tokio::test]
  async fn split_tracks_converts_unsupported_subtitle_codecs() {
    let source = movtext_fixture_path();
    let out = tempfile::tempdir().expect("temp output dir");
    let server = JobServer::new(1).await;

    let response = server
      .split(request(&source, out.path()))
      .await
      .expect("split must succeed");

    assert_eq!(response.tracks.len(), 3);
    assert_eq!(response.warnings.len(), 1, "warnings: {response:?}");
    assert!(
      response.warnings[0].contains("mov_text")
        && response.warnings[0].contains("converting to srt"),
      "got: {}",
      response.warnings[0]
    );

    let subs = &response.tracks[2];
    assert_eq!(
      Path::new(&subs.path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap(),
      "subtitles-0.mks"
    );
    assert_eq!(subs.codec, "srt", "descriptor reports the output codec");

    let probed = dionysus_ffprobe::probe(&subs.path).await.unwrap();
    let s = probed.subtitle_streams().next().expect("subtitle stream");
    // The conversion worked: mov_text did not survive into the .mks — ffprobe
    // labels Matroska's S_TEXT/UTF8 (our srt output) as "subrip".
    assert_eq!(s.codec_name.as_deref(), Some("subrip"));
  }

  #[tokio::test]
  async fn split_tracks_rejects_bad_requests() {
    let server = JobServer::new(1).await;
    let out = tempfile::tempdir().expect("temp output dir");

    let mut req = request(Path::new(""), out.path());
    let err = server.split(req.clone()).await.expect_err("empty source");
    assert_eq!(err.code(), tonic::Code::InvalidArgument);

    req.source_path = "/nonexistent/dionysus/split/test.mkv".to_string();
    let err = server.split(req).await.expect_err("missing source");
    assert_eq!(err.code(), tonic::Code::NotFound);
  }
}
