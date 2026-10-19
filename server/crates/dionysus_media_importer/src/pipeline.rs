//! Per-file import: probe → `SplitTracks` on the encoder → verify every
//! output with ffprobe → insert `media` + `media_track` rows in one
//! transaction → delete the original → enrich with provider metadata.
//! Everything after the commit is best-effort: problems become warnings,
//! never import failures.

use std::path::{Path, PathBuf};

use diesel::prelude::*;
use tonic::transport::Channel;
use uuid::Uuid;

use dionysus_db::{
  models::{NewMedia, NewMediaTrack},
  schema,
  types::{media_kind::MediaKind, media_track_type::MediaTrackType},
};
use dionysus_ffprobe::CodecType;

use crate::config::ImportConfig;
use crate::metadata::MetadataStep;
use crate::proto::dionysus::encoder::v1::{
  job_service_client::JobServiceClient, MediaTrackType as ProtoTrackType, SplitTrack,
  SplitTracksRequest,
};

/// Split is lossless, so a video/audio output's duration must agree with the
/// source within max(2s, 2%) — container timestamp slop is fine, truncation
/// is not.
pub fn duration_matches(source: f64, output: f64) -> bool {
  (source - output).abs() <= (source * 0.02).max(2.0)
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
  #[error("i/o: {0}")]
  Io(#[from] std::io::Error),
  #[error(transparent)]
  Probe(#[from] dionysus_ffprobe::ProbeError),
  #[error("encoder rpc failed: {0}")]
  Rpc(#[from] tonic::Status),
  #[error("cannot reach encoder: {0}")]
  Transport(#[from] tonic::transport::Error),
  #[error("invalid encoder endpoint: {0}")]
  Endpoint(String),
  #[error(transparent)]
  Db(#[from] diesel::result::Error),
  #[error(transparent)]
  Scan(#[from] crate::scanner::ScanError),
  #[error("verification failed for {path}: {reason}")]
  Verify { path: String, reason: String },
  #[error("encoder produced no video track for {0}")]
  NoVideoTrack(PathBuf),
}

#[derive(Debug)]
pub enum FileOutcome {
  /// Fresh import: rows committed, original deleted (or kept with a warning).
  Imported {
    media_id: Uuid,
    track_count: usize,
    warnings: Vec<String>,
  },
  /// Same content already in the library (`source_hash` hit); the leftover
  /// original was deleted.
  Duplicate { media_id: Uuid },
}

/// Serializable result of a directory scan, served as JSON by `serve`.
#[derive(Debug, Default, serde::Serialize)]
pub struct ScanSummary {
  pub imported: Vec<ImportedEntry>,
  pub duplicates: Vec<String>,
  pub failed: Vec<FailedEntry>,
}

#[derive(Debug, serde::Serialize)]
pub struct ImportedEntry {
  pub path: String,
  pub media_id: Uuid,
  pub track_count: usize,
  pub warnings: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct FailedEntry {
  pub path: String,
  pub error: String,
}

pub struct Importer {
  conn: PgConnection,
  encoder: JobServiceClient<Channel>,
  media_root: PathBuf,
  metadata: MetadataStep,
}

impl Importer {
  pub async fn connect(config: &ImportConfig) -> Result<Self, ImportError> {
    let endpoint = Channel::from_shared(config.encoder_addr.clone())
      .map_err(|e| ImportError::Endpoint(e.to_string()))?;
    let channel = endpoint.connect().await?;
    tokio::fs::create_dir_all(&config.media_root).await?;
    Ok(Self {
      conn: dionysus_db::establish_connection(),
      encoder: JobServiceClient::new(channel),
      media_root: config.media_root.clone(),
      metadata: MetadataStep::from_env(&config.media_root),
    })
  }

  /// Import every video container under `dir`, one file at a time. A failing
  /// file is recorded in the summary and never aborts the rest of the scan.
  pub async fn import_directory(&mut self, dir: &Path) -> ScanSummary {
    let mut summary = ScanSummary::default();
    let files = match crate::scanner::scan(dir) {
      Ok(files) => files,
      Err(err) => {
        summary.failed.push(FailedEntry {
          path: dir.display().to_string(),
          error: err.to_string(),
        });
        return summary;
      }
    };

    // Never re-import our own library, even when pointed straight at it.
    let media_root = self.media_root.canonicalize().ok();

    for file in files {
      if let Some(root) = &media_root
        && file
          .path
          .canonicalize()
          .ok()
          .is_some_and(|p| p.starts_with(root))
      {
        tracing::debug!(path = %file.path.display(), "skipping file inside media root");
        continue;
      }
      let path_display = file.path.display().to_string();
      match self.import_file(&file.path).await {
        Ok(FileOutcome::Imported {
          media_id,
          track_count,
          warnings,
        }) => {
          tracing::info!(%media_id, track_count, path = %path_display, "imported");
          summary.imported.push(ImportedEntry {
            path: path_display,
            media_id,
            track_count,
            warnings,
          });
        }
        Ok(FileOutcome::Duplicate { media_id }) => {
          tracing::info!(%media_id, path = %path_display, "already imported, removed original");
          summary.duplicates.push(path_display);
        }
        Err(err) => {
          tracing::error!(path = %path_display, %err, "import failed");
          summary.failed.push(FailedEntry {
            path: path_display,
            error: err.to_string(),
          });
        }
      }
    }
    summary
  }

  pub async fn import_file(&mut self, path: &Path) -> Result<FileOutcome, ImportError> {
    let hash = crate::scanner::hash_file(path)?;
    if let Some(media_id) = self.media_id_by_hash(&hash)? {
      // Identical content is already in the library; the leftover original
      // can go — the DB hash keeps future scans deduped either way.
      if let Err(err) = tokio::fs::remove_file(path).await {
        tracing::warn!(path = %path.display(), %err, "duplicate found but original could not be deleted");
      }
      return Ok(FileOutcome::Duplicate { media_id });
    }

    let probe = dionysus_ffprobe::probe(path).await?;
    let source_duration = probe.duration_secs();
    let parsed = crate::parser::parse_file_name(path);

    let media_id = Uuid::new_v4();
    let dir_name = media_id.to_string();
    let out_dir = self.media_root.join(&dir_name);

    let (tracks, mut warnings) =
      match self.split_and_verify(path, &out_dir, source_duration).await {
        Ok(result) => result,
        Err(err) => {
          // Leave nothing half-imported on disk; the original stays put.
          if let Err(cleanup) = tokio::fs::remove_dir_all(&out_dir).await {
            tracing::warn!(dir = %out_dir.display(), %cleanup, "failed to clean up after import error");
          }
          return Err(err);
        }
      };

    self.insert_media(&tracks, &parsed, &hash, path, media_id, source_duration)?;

    // The rows are committed — from here on, losing the original is an
    // inconvenience, never a failure.
    if let Err(err) = tokio::fs::remove_file(path).await {
      tracing::warn!(path = %path.display(), %err, "imported, but deleting the original failed");
      warnings.push(format!("original not deleted: {err}"));
    }

    // Metadata is post-commit enrichment: a provider outage or a long
    // image download must not undo a good import.
    match self.metadata.enrich(&mut self.conn, media_id, &parsed).await {
      Ok(metadata_warnings) => warnings.extend(metadata_warnings),
      Err(err) => {
        tracing::warn!(%media_id, %err, "metadata enrichment failed");
        warnings.push(format!("metadata: {err}"));
      }
    }

    Ok(FileOutcome::Imported {
      media_id,
      track_count: tracks.len(),
      warnings,
    })
  }

  async fn split_and_verify(
    &mut self,
    source: &Path,
    out_dir: &Path,
    source_duration: Option<f64>,
  ) -> Result<(Vec<SplitTrack>, Vec<String>), ImportError> {
    tokio::fs::create_dir_all(out_dir).await?;
    let response = self
      .encoder
      .split_tracks(SplitTracksRequest {
        id: None,
        source_path: source.display().to_string(),
        output_dir: out_dir.display().to_string(),
      })
      .await?
      .into_inner();

    if !response
      .tracks
      .iter()
      .any(|t| t.track_type == ProtoTrackType::Video as i32)
    {
      return Err(ImportError::NoVideoTrack(source.to_path_buf()));
    }
    for track in &response.tracks {
      verify_track(track, source_duration).await?;
    }
    Ok((response.tracks, response.warnings))
  }

  /// `media` + `media_track` rows in a single transaction — either the whole
  /// file is in the database, or none of it is.
  fn insert_media(
    &mut self,
    tracks: &[SplitTrack],
    parsed: &crate::parser::ParsedName,
    hash: &str,
    source: &Path,
    media_id: Uuid,
    source_duration: Option<f64>,
  ) -> Result<(), ImportError> {
    let dir_name = media_id.to_string();
    // The video track doubles as `manifest_path` until the media server
    // switches to per-track DB lookup (Phase 7).
    let manifest_path = tracks
      .iter()
      .find(|t| t.track_type == ProtoTrackType::Video as i32)
      .map(file_name_of)
      .unwrap_or_default();
    let manifest_path = format!("{dir_name}/{manifest_path}");

    let rows: Vec<(MediaTrackType, &SplitTrack, String)> = tracks
      .iter()
      .map(|t| {
        Ok((
          track_type_from_proto(t.track_type)?,
          t,
          format!("{dir_name}/{}", file_name_of(t)),
        ))
      })
      .collect::<Result<_, ImportError>>()?;

    let kind = if parsed.is_episode() {
      MediaKind::Episode
    } else {
      MediaKind::Movie
    };
    let original_path = source.display().to_string();
    let duration_ms = source_duration.map(|secs| (secs * 1000.0).round() as i64);

    // Blocking Diesel call inside async code: the pipeline imports files
    // sequentially and each transaction is a handful of tiny inserts.
    self
      .conn
      .transaction::<(), diesel::result::Error, _>(|conn| {
        diesel::insert_into(schema::media::table)
          .values(NewMedia {
            id: media_id,
            manifest_path: &manifest_path,
            kind,
            source_hash: Some(hash),
            duration_ms,
            release_year: parsed.year,
            season_number: parsed.season,
            episode_number: parsed.episode,
            original_path: Some(&original_path),
          })
          .execute(conn)?;

        for (track_type, track, storage_path) in &rows {
          diesel::insert_into(schema::media_track::table)
            .values(NewMediaTrack {
              media_id,
              track_type: *track_type,
              locale: &track.language,
              storage_path,
              track_index: track.stream_index,
              codec: Some(&track.codec),
              is_default: track.is_default,
              is_forced: track.is_forced,
              channels: (track.channels > 0).then_some(track.channels as i16),
              width: (track.width > 0).then_some(track.width),
              height: (track.height > 0).then_some(track.height),
              title: (!track.title.is_empty()).then_some(track.title.as_str()),
            })
            .execute(conn)?;
        }
        Ok(())
      })?;
    Ok(())
  }

  fn media_id_by_hash(&mut self, hash: &str) -> Result<Option<Uuid>, ImportError> {
    use schema::media::dsl as m;
    Ok(m::media
      .select(m::id)
      .filter(m::source_hash.eq(hash))
      .first::<Uuid>(&mut self.conn)
      .optional()?)
  }
}

fn file_name_of(track: &SplitTrack) -> String {
  Path::new(&track.path)
    .file_name()
    .map(|n| n.to_string_lossy().into_owned())
    .unwrap_or_default()
}

fn track_type_from_proto(value: i32) -> Result<MediaTrackType, ImportError> {
  match ProtoTrackType::try_from(value) {
    Ok(ProtoTrackType::Video) => Ok(MediaTrackType::Video),
    Ok(ProtoTrackType::Audio) => Ok(MediaTrackType::Audio),
    Ok(ProtoTrackType::Subtitles) => Ok(MediaTrackType::Subtitles),
    _ => Err(ImportError::Verify {
      path: "encoder response".to_string(),
      reason: format!("unspecified track type {value}"),
    }),
  }
}

/// Check one split output against its descriptor: the file must exist with
/// exactly one stream of the promised type, and video/audio containers must
/// keep the source duration (subtitle containers only span their cues).
async fn verify_track(
  track: &SplitTrack,
  source_duration: Option<f64>,
) -> Result<(), ImportError> {
  let expected = match ProtoTrackType::try_from(track.track_type) {
    Ok(ProtoTrackType::Video) => CodecType::Video,
    Ok(ProtoTrackType::Audio) => CodecType::Audio,
    Ok(ProtoTrackType::Subtitles) => CodecType::Subtitle,
    _ => {
      return Err(ImportError::Verify {
        path: track.path.clone(),
        reason: format!("unspecified track type {}", track.track_type),
      });
    }
  };
  let probe = dionysus_ffprobe::probe(&track.path).await?;
  let matching = probe
    .streams
    .iter()
    .filter(|s| s.codec_type == expected)
    .count();
  if matching != 1 {
    return Err(ImportError::Verify {
      path: track.path.clone(),
      reason: format!("expected exactly one {expected:?} stream, found {matching}"),
    });
  }
  if expected != CodecType::Subtitle
    && let (Some(source_duration), Some(output_duration)) =
      (source_duration, probe.duration_secs())
    && !duration_matches(source_duration, output_duration)
  {
    return Err(ImportError::Verify {
      path: track.path.clone(),
      reason: format!(
        "duration {output_duration:.2}s does not match source {source_duration:.2}s"
      ),
    });
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use dionysus_db::models::{Media, MediaTrack};

  #[test]
  fn duration_tolerance_allows_slop_but_not_truncation() {
    assert!(duration_matches(6.0, 6.016)); // header/timestamp slop
    assert!(duration_matches(3600.0, 3599.5));
    assert!(!duration_matches(3600.0, 3000.0)); // truncated
    assert!(!duration_matches(120.0, 130.0)); // overstated
  }

  /// Requires a migrated database (CI provides one); skipped locally when
  /// DATABASE_URL is unset.
  #[test]
  fn inserts_media_and_tracks_roundtrip() {
    if std::env::var("DATABASE_URL").is_err() {
      eprintln!("skipping inserts_media_and_tracks_roundtrip: DATABASE_URL is not set");
      return;
    }
    let mut conn = dionysus_db::establish_connection();
    dionysus_db::run_migrations(&mut conn).expect("migrations must run");

    let media_id = Uuid::new_v4();
    let test_hash = format!("test-hash-{media_id}");
    diesel::insert_into(schema::media::table)
      .values(NewMedia {
        id: media_id,
        manifest_path: "test-media/video-0.mkv",
        kind: MediaKind::Episode,
        source_hash: Some(&test_hash),
        duration_ms: Some(6_000),
        release_year: Some(2024),
        season_number: Some(2),
        episode_number: Some(5),
        original_path: Some("/imports/Test.Show.S02E05.mkv"),
      })
      .execute(&mut conn)
      .expect("insert media");
    diesel::insert_into(schema::media_track::table)
      .values(NewMediaTrack {
        media_id,
        track_type: MediaTrackType::Audio,
        locale: "spa",
        storage_path: "test-media/audio-0.mka",
        track_index: 2,
        codec: Some("aac"),
        is_default: false,
        is_forced: false,
        channels: Some(2),
        width: None,
        height: None,
        title: Some("Commentary"),
      })
      .execute(&mut conn)
      .expect("insert track");

    let media: Media = schema::media::table
      .find(media_id)
      .first(&mut conn)
      .expect("media row");
    assert_eq!(media.kind, MediaKind::Episode);
    assert_eq!(media.release_year, Some(2024));
    assert_eq!((media.season_number, media.episode_number), (Some(2), Some(5)));
    assert_eq!(media.source_hash.as_deref(), Some(test_hash.as_str()));
    assert_eq!(media.duration_ms, Some(6_000));

    let tracks = MediaTrack::belonging_to(&media)
      .load::<MediaTrack>(&mut conn)
      .expect("track rows");
    assert_eq!(tracks.len(), 1);
    let track = &tracks[0];
    assert_eq!(track.track_type, MediaTrackType::Audio);
    assert_eq!(track.locale, "spa");
    assert_eq!(track.storage_path, "test-media/audio-0.mka");
    assert_eq!(track.track_index, 2);
    assert_eq!(track.codec.as_deref(), Some("aac"));
    assert!(!track.is_default && !track.is_forced);
    assert_eq!(track.channels, Some(2));
    assert_eq!(track.title.as_deref(), Some("Commentary"));

    // Cleanup (cascade-checks the FK while we're at it).
    diesel::delete(schema::media::table.find(media_id))
      .execute(&mut conn)
      .expect("cleanup");
  }
}
