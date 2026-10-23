//! Where a media id's playable file lives. The database is authoritative:
//! the default track of the requested type wins, lowest stream index on
//! ties. The directory scan only serves legacy rows and deployments
//! without a database.

use std::path::{Path, PathBuf};

use diesel::prelude::*;
use uuid::Uuid;

use dionysus_db::types::media_track_type::MediaTrackType;

/// Source file to read for `media_id` and the requested track type: the
/// preferred `media_track` row, or the scanned media directory as fallback.
pub async fn resolve_source(
  media_root: &Path,
  media_id: &Uuid,
  track_type: MediaTrackType,
) -> Option<PathBuf> {
  if let Some(path) = track_path(media_root, *media_id, track_type).await
    && tokio::fs::try_exists(&path).await.unwrap_or(false)
  {
    return Some(path);
  }
  scan_source(media_root, media_id).await
}

/// `storage_path` of the preferred track: default first, then the lowest
/// stream index. A database outage degrades to `None` (the scan fallback).
async fn track_path(
  media_root: &Path,
  media_id: Uuid,
  track_type: MediaTrackType,
) -> Option<PathBuf> {
  let media_root = media_root.to_path_buf();
  tokio::task::spawn_blocking(move || {
    let mut conn = dionysus_db::try_establish_connection()?;
    use dionysus_db::schema::media_track::dsl as track;
    let storage_path: Option<String> = track::media_track
      .filter(track::media_id.eq(media_id))
      .filter(track::track_type.eq(track_type))
      .order((track::is_default.desc(), track::track_index.asc()))
      .select(track::storage_path)
      .first(&mut conn)
      .optional()
      .ok()
      .flatten();
    storage_path.map(|path| media_root.join(path))
  })
  .await
  .ok()
  .flatten()
}

/// Legacy layout: the first regular file in the media directory, preferring
/// common video container extensions.
async fn scan_source(media_root: &Path, media_id: &Uuid) -> Option<PathBuf> {
  let dir = media_root.join(media_id.to_string());
  let mut entries = tokio::fs::read_dir(&dir).await.ok()?;
  let mut fallback: Option<PathBuf> = None;
  while let Ok(Some(entry)) = entries.next_entry().await {
    let path = entry.path();
    if !entry
      .file_type()
      .await
      .map(|t| t.is_file())
      .unwrap_or(false)
    {
      continue;
    }
    let preferred = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
      matches!(
        e.to_lowercase().as_str(),
        "mp4" | "mkv" | "webm" | "mov" | "m4v" | "ts"
      )
    });
    if preferred {
      return Some(path);
    }
    if fallback.is_none() {
      fallback = Some(path);
    }
  }
  fallback
}

#[cfg(test)]
mod tests {
  use super::*;

  fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("dionysus-source-{name}-{}", Uuid::new_v4()))
  }

  #[tokio::test]
  async fn scans_the_media_directory_without_database_rows() {
    let root = temp_root("scan");
    let media_id = Uuid::new_v4();
    let dir = root.join(media_id.to_string());
    tokio::fs::create_dir_all(&dir).await.expect("temp dir");
    tokio::fs::write(dir.join("notes.txt"), b"x").await.expect("write");
    tokio::fs::write(dir.join("source.MKV"), b"x").await.expect("write");

    let resolved = resolve_source(&root, &media_id, MediaTrackType::Video).await;
    assert_eq!(resolved, Some(dir.join("source.MKV")));

    tokio::fs::remove_dir_all(&root).await.expect("cleanup");
  }

  /// Requires a migrated database (CI provides one); skipped locally when
  /// DATABASE_URL is unset.
  #[tokio::test]
  async fn prefers_the_default_track_from_the_database() {
    if std::env::var("DATABASE_URL").is_err() {
      eprintln!("skipping prefers_the_default_track_from_the_database: DATABASE_URL is not set");
      return;
    }
    let mut conn = dionysus_db::establish_connection();
    dionysus_db::run_migrations(&mut conn).expect("migrations must run");

    let media_id = Uuid::new_v4();
    let root = temp_root("db");
    let dir = root.join(media_id.to_string());
    tokio::fs::create_dir_all(&dir).await.expect("temp dir");
    tokio::fs::write(dir.join("video-0.mkv"), b"x").await.expect("write");
    tokio::fs::write(dir.join("video-1.mkv"), b"x").await.expect("write");
    tokio::fs::write(dir.join("audio-0.mka"), b"x").await.expect("write");

    use dionysus_db::models::{NewMedia, NewMediaTrack};
    use dionysus_db::types::media_kind::MediaKind;
    diesel::insert_into(dionysus_db::schema::media::table)
      .values(NewMedia {
        id: media_id,
        manifest_path: "source-db-test/video-0.mkv",
        kind: MediaKind::Movie,
        source_hash: None,
        duration_ms: None,
        release_year: None,
        season_number: None,
        episode_number: None,
        original_path: None,
      })
      .execute(&mut conn)
      .expect("insert media");
    let tracks = [
      (MediaTrackType::Video, "video-0.mkv", false, 0),
      (MediaTrackType::Video, "video-1.mkv", true, 1),
      (MediaTrackType::Audio, "audio-0.mka", false, 0),
    ];
    for (track_type, file, is_default, track_index) in tracks {
      diesel::insert_into(dionysus_db::schema::media_track::table)
        .values(NewMediaTrack {
          media_id,
          track_type,
          locale: "und",
          storage_path: &format!("{media_id}/{file}"),
          track_index,
          codec: None,
          is_default,
          is_forced: false,
          channels: None,
          width: None,
          height: None,
          title: None,
        })
        .execute(&mut conn)
        .expect("insert track");
    }

    let video = resolve_source(&root, &media_id, MediaTrackType::Video).await;
    assert_eq!(video, Some(dir.join("video-1.mkv")), "default video track wins");
    let audio = resolve_source(&root, &media_id, MediaTrackType::Audio).await;
    assert_eq!(audio, Some(dir.join("audio-0.mka")));

    diesel::delete(dionysus_db::schema::media::table.find(media_id))
      .execute(&mut conn)
      .expect("cleanup");
    tokio::fs::remove_dir_all(&root).await.expect("cleanup files");
  }
}
