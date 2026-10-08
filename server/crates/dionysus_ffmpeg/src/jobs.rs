use std::sync::Arc;

use chrono::{DateTime, Datelike, Timelike, Utc};
use rust_ffmpeg::{
  Capabilities, Codec, CodecOptions, FFmpegBuilder, Input, Output, StreamMap, VideoFilter,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tonic::{Request, Response, Status};

use crate::codec::{
  apply_rate_control, filter_flag, platform_from_job, resolution_height, select_audio_encoder,
  select_encoder, width_for_height,
};
// Prost names oneof enums after the `oneof` label; alias them back to the
// `*OneOf` convention so they don't shadow the real codec enums.
use crate::proto::dionysus::encoder::v1::audio_job::AudioCodec as AudioCodecOneOf;
use crate::proto::dionysus::encoder::v1::video_job::VideoCodec as VideoCodecOneOf;
use crate::proto::dionysus::encoder::v1::{
  AudioCodec, AudioJob, JobResult, SegmentAudioRequest, SegmentAudioResponse, SegmentVideoRequest,
  SegmentVideoResponse, VideoCodec, VideoJob, job_service_server::JobService,
};

pub struct JobServer {
  permits: Arc<Semaphore>,
  caps: Capabilities,
}

impl JobServer {
  /// Build a server that runs at most `max_concurrent` ffmpeg jobs at once.
  /// Extra `SegmentVideo`/`SegmentAudio` calls wait for a permit (unbounded FIFO queue).
  pub async fn new(max_concurrent: usize) -> Self {
    let max = max_concurrent.max(1);
    let caps = rust_ffmpeg::capabilities().await.unwrap_or_default();
    Self {
      permits: Arc::new(Semaphore::new(max)),
      caps,
    }
  }

  fn timestamp_to_chrono(ts: &prost_types::Timestamp) -> DateTime<Utc> {
    // Clamp pre-epoch to UNIX_EPOCH and nanos into from_timestamp's valid range.
    DateTime::from_timestamp(ts.seconds.max(0), ts.nanos.clamp(0, 999_999_999) as u32)
      .unwrap_or(DateTime::UNIX_EPOCH)
  }

  /// Resolve `start`/`end` timestamps into an output seek point and clip length.
  fn window(
    start: Option<&prost_types::Timestamp>,
    end: Option<&prost_types::Timestamp>,
  ) -> Result<(Option<DateTime<Utc>>, Option<chrono::Duration>), Status> {
    let seek = start.map(Self::timestamp_to_chrono);
    let end = end.map(Self::timestamp_to_chrono);
    let clip_len = match (seek, end) {
      (Some(s), Some(e)) => {
        if e <= s {
          return Err(Status::invalid_argument("job.end must be after job.start"));
        }
        Some(e - s)
      }
      (None, Some(e)) => Some(e.signed_duration_since(DateTime::UNIX_EPOCH)),
      (Some(_), None) | (None, None) => None,
    };
    Ok((seek, clip_len))
  }

  /// Probe source dimensions with ffprobe: `ffprobe -v error -select_streams v:0
  /// -show_entries stream=width,height -of csv=p=0:s=x <path>`.
  async fn probe_dimensions(path: &str) -> Result<(u32, u32), Status> {
    let out = tokio::process::Command::new("ffprobe")
      .args([
        "-v",
        "error",
        "-select_streams",
        "v:0",
        "-show_entries",
        "stream=width,height",
        "-of",
        "csv=p=0:s=x",
        path,
      ])
      .output()
      .await
      .map_err(|e| Status::internal(format!("failed to run ffprobe: {e}")))?;

    if !out.status.success() {
      let stderr = String::from_utf8_lossy(&out.stderr);
      return Err(Status::internal(format!(
        "ffprobe failed: {}",
        stderr.trim()
      )));
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let text = text.trim();
    let (w, h) = text
      .split_once(['x', 'X'])
      .ok_or_else(|| Status::internal(format!("cannot parse ffprobe dimensions: {text:?}")))?;
    let w: u32 = w
      .trim()
      .parse()
      .map_err(|_| Status::internal(format!("bad ffprobe width: {text:?}")))?;
    let h: u32 = h
      .trim()
      .parse()
      .map_err(|_| Status::internal(format!("bad ffprobe height: {text:?}")))?;
    if w == 0 || h == 0 {
      return Err(Status::internal(format!(
        "ffprobe returned empty dimensions: {text:?}"
      )));
    }
    Ok((w, h))
  }

  fn resolve_video_encoder(&self, job: &VideoJob) -> String {
    match &job.video_codec {
      Some(VideoCodecOneOf::CustomVideoCodec(s)) if !s.trim().is_empty() => s.clone(),
      Some(VideoCodecOneOf::KnownVideoCodec(v)) => {
        let codec = VideoCodec::try_from(*v).unwrap_or(VideoCodec::Unspecified);
        let codec = match codec {
          VideoCodec::Unspecified => VideoCodec::H264,
          c => c,
        };
        let platform = platform_from_job(job.hardware, job.hardware_accelerator.as_deref());
        select_encoder(codec, platform, &self.caps)
      }
      _ => {
        let platform = platform_from_job(job.hardware, job.hardware_accelerator.as_deref());
        select_encoder(VideoCodec::H264, platform, &self.caps)
      }
    }
  }

  fn resolve_audio_encoder(&self, job: &AudioJob) -> String {
    match &job.audio_codec {
      Some(AudioCodecOneOf::CustomAudioCodec(s)) if !s.trim().is_empty() => s.clone(),
      Some(AudioCodecOneOf::KnownAudioCodec(v)) => {
        let codec = AudioCodec::try_from(*v).unwrap_or(AudioCodec::Unspecified);
        select_audio_encoder(codec, &self.caps)
      }
      _ => select_audio_encoder(AudioCodec::Aac, &self.caps),
    }
  }

  /// Transcode one `[start, end)` window of `job.path` to a video-only
  /// fragmented MP4 chunk.
  async fn transcode_video(&self, job: VideoJob) -> Result<JobResult, Status> {
    if job.path.trim().is_empty() {
      return Err(Status::invalid_argument("video job.path is empty"));
    }
    if tokio::fs::metadata(&job.path).await.is_err() {
      return Err(Status::not_found(format!(
        "video file not found: {}",
        job.path
      )));
    }

    let (seek, clip_len) = Self::window(job.start.as_ref(), job.end.as_ref())?;
    let video_encoder = self.resolve_video_encoder(&job);

    // Resolution: target height from enum, width from source aspect ratio.
    let scale_filter: Option<VideoFilter> = match resolution_height(job.rescaling_resolution) {
      None => None,
      Some(target_h) => {
        if video_encoder == "copy" {
          return Err(Status::invalid_argument(
            "cannot rescale with VIDEO_CODEC_COPY; pick a real encoder",
          ));
        }
        let (src_w, src_h) = Self::probe_dimensions(&job.path).await?;
        let w = width_for_height(src_w, src_h, target_h);
        let flag = filter_flag(job.rescaling_filter);
        Some(
          VideoFilter::new("scale")
            .param("w", w)
            .param("h", target_h)
            .param("flags", flag),
        )
      }
    };

    let mut builder =
      FFmpegBuilder::new().map_err(|e| Status::internal(format!("ffmpeg not available: {e}")))?;

    if let Some(acc) = job.hardware_accelerator.as_deref()
      && !acc.trim().is_empty()
    {
      builder = builder.hwaccel(acc);
    }

    // Video-only output: map just the video stream of the single input.
    builder = builder
      .input(Input::new(job.path.clone()))
      .map(StreamMap::video_from(0));

    if let Some(f) = scale_filter {
      builder = builder.video_filter(f);
    }

    // Fragmented MP4 into a temp file; bytes are read back into JobResult.data.
    let (tmp, out_path) = Self::temp_out()?;
    let mut output = Output::new(out_path.clone())
      .format("mp4")
      .movflags("frag_keyframe+empty_moov+default_base_moof")
      .avoid_negative_ts("make_zero");

    // Accurate output-side seeking for frame-exact chunk extraction.
    // chrono time is converted to std time only here, at the ffmpeg boundary.
    if let Some(s) = seek {
      let offset = s
        .signed_duration_since(DateTime::UNIX_EPOCH)
        .to_std()
        .map_err(|_| Status::invalid_argument("job.start is before the epoch"))?;
      output = output.start_time(offset.into());
    }
    if let Some(len) = clip_len {
      let len = len
        .to_std()
        .map_err(|_| Status::internal("negative clip length"))?;
      output = output.duration(len.into());
    }

    let mut vopts = CodecOptions::new(Codec::new(video_encoder.clone()));
    if video_encoder != "copy" {
      vopts = apply_rate_control(vopts, job.video_bitrate.as_ref());
    }
    output = output.video_codec_opts(vopts);

    builder = builder.output(output).overwrite();

    tracing::info!(
      video = %job.path,
      vcodec = %video_encoder,
      "starting ffmpeg video segment"
    );

    let bytes = Self::run(builder, &out_path).await?;
    drop(tmp);

    Ok(JobResult {
      data: bytes,
      codec_used: video_encoder,
      cached: false,
      created_at: Some(now_datetime()),
    })
  }

  /// Transcode one `[start, end)` window of `job.path` to an audio-only
  /// fragmented MP4 chunk. `job.path` may be a muxed source; only the audio
  /// stream is mapped.
  async fn transcode_audio(&self, job: AudioJob) -> Result<JobResult, Status> {
    if job.path.trim().is_empty() {
      return Err(Status::invalid_argument("audio job.path is empty"));
    }
    if tokio::fs::metadata(&job.path).await.is_err() {
      return Err(Status::not_found(format!(
        "audio file not found: {}",
        job.path
      )));
    }

    let (seek, clip_len) = Self::window(job.start.as_ref(), job.end.as_ref())?;
    let audio_encoder = self.resolve_audio_encoder(&job);

    let mut builder =
      FFmpegBuilder::new().map_err(|e| Status::internal(format!("ffmpeg not available: {e}")))?;

    // Audio-only output: map just the audio stream of the single input, so a
    // muxed source is demuxed here.
    builder = builder
      .input(Input::new(job.path.clone()))
      .map(StreamMap::audio_from(0));

    // Fragmented MP4 into a temp file; bytes are read back into JobResult.data.
    // `frag_keyframe` is video-driven: with an audio-only output the muxer
    // flushes one fragment at EOF, i.e. exactly one chunk per window.
    let (tmp, out_path) = Self::temp_out()?;
    let mut output = Output::new(out_path.clone())
      .format("mp4")
      .movflags("frag_keyframe+empty_moov+default_base_moof")
      .avoid_negative_ts("make_zero");

    if let Some(s) = seek {
      let offset = s
        .signed_duration_since(DateTime::UNIX_EPOCH)
        .to_std()
        .map_err(|_| Status::invalid_argument("job.start is before the epoch"))?;
      output = output.start_time(offset.into());
    }
    if let Some(len) = clip_len {
      let len = len
        .to_std()
        .map_err(|_| Status::internal("negative clip length"))?;
      output = output.duration(len.into());
    }

    let mut aopts = CodecOptions::new(Codec::new(audio_encoder.clone()));
    if audio_encoder != "copy" {
      aopts = apply_rate_control(aopts, job.audio_bitrate.as_ref());
    }
    output = output.audio_codec_opts(aopts);

    builder = builder.output(output).overwrite();

    tracing::info!(
      audio = %job.path,
      acodec = %audio_encoder,
      "starting ffmpeg audio segment"
    );

    let bytes = Self::run(builder, &out_path).await?;
    drop(tmp);

    Ok(JobResult {
      data: bytes,
      codec_used: audio_encoder,
      cached: false,
      created_at: Some(now_datetime()),
    })
  }

  fn temp_out() -> Result<(tempfile::NamedTempFile, String), Status> {
    let tmp = tempfile::Builder::new()
      .suffix(".mp4")
      .tempfile()
      .map_err(|e| Status::internal(format!("cannot create temp output: {e}")))?;
    let path = tmp.path().to_string_lossy().into_owned();
    Ok((tmp, path))
  }

  async fn run(builder: FFmpegBuilder, out_path: &str) -> Result<Vec<u8>, Status> {
    builder.run().await.map_err(|e| {
      let msg = e.to_string();
      let tail: String = msg
        .chars()
        .rev()
        .take(2000)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
      Status::internal(format!("ffmpeg failed: {tail}"))
    })?;

    let bytes = tokio::fs::read(out_path)
      .await
      .map_err(|e| Status::internal(format!("cannot read ffmpeg output: {e}")))?;
    Ok(bytes)
  }

  /// Queue: at most N jobs hold a permit; the rest wait here.
  async fn acquire_permit(&self) -> Result<OwnedSemaphorePermit, Status> {
    self
      .permits
      .clone()
      .acquire_owned()
      .await
      .map_err(|_| Status::unavailable("encoder queue is shutting down"))
  }
}

fn now_datetime() -> crate::proto::google::r#type::DateTime {
  use crate::proto::google::r#type::{DateTime, date_time::TimeOffset};
  let now = Utc::now();
  DateTime {
    year: now.date_naive().year(),
    month: now.date_naive().month() as i32,
    day: now.date_naive().day() as i32,
    hours: now.time().hour() as i32,
    minutes: now.time().minute() as i32,
    seconds: now.time().second() as i32,
    nanos: now.timestamp_subsec_nanos() as i32,
    time_offset: Some(TimeOffset::UtcOffset(prost_types::Duration {
      seconds: 0,
      nanos: 0,
    })),
  }
}

#[tonic::async_trait]
impl JobService for JobServer {
  async fn segment_video(
    &self,
    request: Request<SegmentVideoRequest>,
  ) -> Result<Response<SegmentVideoResponse>, Status> {
    let job = request
      .into_inner()
      .info
      .ok_or_else(|| Status::invalid_argument("SegmentVideoRequest.info is missing"))?;

    let permit = self.acquire_permit().await?;
    let result = self.transcode_video(job).await?;
    drop(permit);

    Ok(Response::new(SegmentVideoResponse {
      result: Some(result),
    }))
  }

  async fn segment_audio(
    &self,
    request: Request<SegmentAudioRequest>,
  ) -> Result<Response<SegmentAudioResponse>, Status> {
    let job = request
      .into_inner()
      .info
      .ok_or_else(|| Status::invalid_argument("SegmentAudioRequest.info is missing"))?;

    let permit = self.acquire_permit().await?;
    let result = self.transcode_audio(job).await?;
    drop(permit);

    Ok(Response::new(SegmentAudioResponse {
      result: Some(result),
    }))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proto::dionysus::encoder::v1::{Filter, Resolution};

  #[test]
  fn timestamp_to_chrono_clamps() {
    let ts = prost_types::Timestamp {
      seconds: 90,
      nanos: 500_000_000,
    };
    let dt = JobServer::timestamp_to_chrono(&ts);
    assert_eq!(dt, DateTime::from_timestamp(90, 500_000_000).unwrap());

    // Pre-epoch clamps to UNIX_EPOCH.
    let ts = prost_types::Timestamp {
      seconds: -5,
      nanos: -100,
    };
    assert_eq!(JobServer::timestamp_to_chrono(&ts), DateTime::UNIX_EPOCH);

    // Out-of-range nanos clamp into from_timestamp's valid range.
    let ts = prost_types::Timestamp {
      seconds: 3,
      nanos: 2_000_000_000,
    };
    assert_eq!(
      JobServer::timestamp_to_chrono(&ts),
      DateTime::from_timestamp(3, 999_999_999).unwrap()
    );
  }

  fn fixture_path() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("dionysus_ffmpeg_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample_1080p.mp4");
    if path.exists() {
      return path;
    }
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
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-shortest",
      ])
      .arg(&path)
      .arg("-y")
      .status()
      .expect("ffmpeg must be installed for tests");
    assert!(status.success(), "fixture generation failed");
    path
  }

  fn window(
    start: i64,
    end: i64,
  ) -> (
    Option<prost_types::Timestamp>,
    Option<prost_types::Timestamp>,
  ) {
    (
      Some(prost_types::Timestamp {
        seconds: start,
        nanos: 0,
      }),
      Some(prost_types::Timestamp {
        seconds: end,
        nanos: 0,
      }),
    )
  }

  /// Stream types (`video`/`audio`) present in a fMP4 blob, via ffprobe.
  async fn stream_types(data: &[u8]) -> Vec<String> {
    let tmp = tempfile::Builder::new()
      .suffix(".mp4")
      .tempfile()
      .expect("temp file");
    tokio::fs::write(tmp.path(), data)
      .await
      .expect("write blob");
    let out = tokio::process::Command::new("ffprobe")
      .args([
        "-v",
        "error",
        "-show_entries",
        "stream=codec_type",
        "-of",
        "csv=p=0",
      ])
      .arg(tmp.path())
      .output()
      .await
      .expect("ffprobe must be installed for tests");
    assert!(out.status.success(), "ffprobe failed on encoder output");
    String::from_utf8_lossy(&out.stdout)
      .lines()
      .map(|l| l.trim().to_string())
      .filter(|l| !l.is_empty())
      .collect()
  }

  #[tokio::test]
  async fn segment_video_extracts_fmp4_chunk_at_target_height() {
    let video = fixture_path();
    let server = JobServer::new(1).await;
    let (start, end) = window(1, 3);
    let job = VideoJob {
      id: None,
      path: video.to_string_lossy().into_owned(),
      hardware_accelerator: None,
      hardware: None,
      rescaling_resolution: Some(Resolution::Resolution480 as i32),
      rescaling_filter: Some(Filter::Bicubic as i32),
      video_codec: Some(VideoCodecOneOf::KnownVideoCodec(VideoCodec::H264 as i32)),
      video_bitrate: None,
      start,
      end,
    };

    let result = server
      .transcode_video(job)
      .await
      .expect("transcode must succeed");
    assert_eq!(result.codec_used, "libx264");
    assert!(!result.cached);
    assert!(result.created_at.is_some());
    // MP4 signature: [size:4][ftyp]
    assert!(result.data.len() > 1_000, "output suspiciously small");
    assert_eq!(&result.data[4..8], b"ftyp");
    // Fragmented output must contain movie-fragment boxes.
    assert!(
      result.data.windows(4).any(|w| w == b"moof"),
      "output is not fragmented mp4 (no moof box)"
    );
    // Video job must not carry an audio track.
    assert_eq!(stream_types(&result.data).await, vec!["video"]);
  }

  #[tokio::test]
  async fn segment_audio_extracts_fmp4_audio_chunk() {
    let source = fixture_path();
    let server = JobServer::new(1).await;
    let (start, end) = window(1, 3);
    let job = AudioJob {
      id: None,
      path: source.to_string_lossy().into_owned(),
      audio_codec: Some(AudioCodecOneOf::KnownAudioCodec(AudioCodec::Aac as i32)),
      audio_bitrate: None,
      start,
      end,
    };

    let result = server
      .transcode_audio(job)
      .await
      .expect("transcode must succeed");
    assert_eq!(result.codec_used, "aac");
    assert!(!result.cached);
    assert!(result.created_at.is_some());
    assert!(result.data.len() > 1_000, "output suspiciously small");
    assert_eq!(&result.data[4..8], b"ftyp");
    assert!(
      result.data.windows(4).any(|w| w == b"moof"),
      "output is not fragmented mp4 (no moof box)"
    );
    // Audio job must demux the muxed fixture down to an audio-only track.
    assert_eq!(stream_types(&result.data).await, vec!["audio"]);
  }
}
