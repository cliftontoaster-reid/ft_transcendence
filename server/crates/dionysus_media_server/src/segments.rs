use rocket::http::{ContentType, Status};
use rocket::State;
use uuid::Uuid;

use crate::bitrate;
use crate::cache;
use crate::config::AppConfig;
use crate::manifest;
use crate::params::{AudioCodecParam, ChannelsParam, Quality, ResolutionParam, SegmentFile, VideoCodecParam};
use crate::proto::dionysus::encoder::v1::{
  audio_job, job_service_client::JobServiceClient, video_job, AudioJob, SegmentAudioRequest,
  SegmentVideoRequest, VideoJob,
};
use crate::source;
use crate::window;

fn mp4() -> ContentType {
  ContentType::new("video", "mp4")
}

fn parse_media_id(raw: &str) -> Result<Uuid, Status> {
  Uuid::parse_str(raw).map_err(|_| Status::BadRequest)
}

fn tonic_status(s: tonic::Status) -> Status {
  use tonic::Code;
  match s.code() {
    Code::InvalidArgument => Status::BadRequest,
    Code::NotFound => Status::NotFound,
    Code::Unavailable => Status::ServiceUnavailable,
    _ => Status::InternalServerError,
  }
}

/// `GET {base}/media/{media_id}/metadata/dash/manifest.mpd`
#[rocket::get("/media/<media_id>/metadata/dash/manifest.mpd")]
pub async fn manifest_route(
  media_id: &str,
  config: &State<AppConfig>,
) -> Result<(ContentType, String), Status> {
  let id = parse_media_id(media_id)?;
  if source::resolve_source(&config.media_root, &id)
    .await
    .is_none()
  {
    return Err(Status::NotFound);
  }
  Ok((
    ContentType::new("application", "dash+xml"),
    manifest::build_manifest(&id, config),
  ))
}

/// `GET {base}/media/{media_id}/video/{resolution}/{codec}/{quality}/{number}.fmp4`
#[rocket::get("/media/<media_id>/video/<resolution>/<codec>/<quality>/<file>")]
pub async fn video_segment(
  media_id: &str,
  resolution: ResolutionParam,
  codec: VideoCodecParam,
  quality: Quality,
  file: SegmentFile,
  config: &State<AppConfig>,
) -> Result<(ContentType, Vec<u8>), Status> {
  let id = parse_media_id(media_id)?;
  let source = source::resolve_source(&config.media_root, &id)
    .await
    .ok_or(Status::NotFound)?;

  let cache_path = config
    .cache_dir
    .join(id.to_string())
    .join("video")
    .join(resolution.0.to_string())
    .join(codec.as_str())
    .join(quality.as_str())
    .join(format!("{}.fmp4", file.0));

  if let Some(bytes) = cache::read_if_fresh(&cache_path, config.cache_ttl_secs).await {
    return Ok((mp4(), bytes));
  }

  let (start, end) = window::window(file.0, config.segment_duration_secs);
  let job = VideoJob {
    id: None,
    path: source.to_string_lossy().into_owned(),
    hardware_accelerator: None,
    hardware: None,
    rescaling_resolution: Some(resolution.proto_value()),
    rescaling_filter: None,
    video_codec: Some(video_job::VideoCodec::KnownVideoCodec(codec.proto_value())),
    video_bitrate: Some(bitrate::video_rate_control(quality, config)),
    start,
    end,
  };

  let mut client = JobServiceClient::connect(config.encoder_addr.clone())
    .await
    .map_err(|_| Status::ServiceUnavailable)?;
  let result = client
    .segment_video(SegmentVideoRequest { info: Some(job) })
    .await
    .map_err(tonic_status)?
    .into_inner()
    .result
    .ok_or(Status::InternalServerError)?;

  if let Err(e) = cache::store(&cache_path, &result.data).await {
    rocket::warn!("segment cache store failed: {e}");
  }
  cache::enforce_size_limit(&config.cache_dir, config.cache_max_bytes).await;

  Ok((mp4(), result.data))
}

/// `GET {base}/media/{media_id}/audio/{channels}/{codec}/{quality}/{number}.fmp4`
#[rocket::get("/media/<media_id>/audio/<channels>/<codec>/<quality>/<file>")]
pub async fn audio_segment(
  media_id: &str,
  channels: ChannelsParam,
  codec: AudioCodecParam,
  quality: Quality,
  file: SegmentFile,
  config: &State<AppConfig>,
) -> Result<(ContentType, Vec<u8>), Status> {
  let id = parse_media_id(media_id)?;
  let source = source::resolve_source(&config.media_root, &id)
    .await
    .ok_or(Status::NotFound)?;

  let cache_path = config
    .cache_dir
    .join(id.to_string())
    .join("audio")
    .join(channels.0.to_string())
    .join(codec.as_str())
    .join(quality.as_str())
    .join(format!("{}.fmp4", file.0));

  if let Some(bytes) = cache::read_if_fresh(&cache_path, config.cache_ttl_secs).await {
    return Ok((mp4(), bytes));
  }

  let (start, end) = window::window(file.0, config.segment_duration_secs);
  let job = AudioJob {
    id: None,
    path: source.to_string_lossy().into_owned(),
    audio_codec: Some(audio_job::AudioCodec::KnownAudioCodec(codec.proto_value())),
    audio_bitrate: Some(bitrate::audio_rate_control(quality, config)),
    start,
    end,
  };

  let mut client = JobServiceClient::connect(config.encoder_addr.clone())
    .await
    .map_err(|_| Status::ServiceUnavailable)?;
  let result = client
    .segment_audio(SegmentAudioRequest { info: Some(job) })
    .await
    .map_err(tonic_status)?
    .into_inner()
    .result
    .ok_or(Status::InternalServerError)?;

  if let Err(e) = cache::store(&cache_path, &result.data).await {
    rocket::warn!("segment cache store failed: {e}");
  }
  cache::enforce_size_limit(&config.cache_dir, config.cache_max_bytes).await;

  Ok((mp4(), result.data))
}
