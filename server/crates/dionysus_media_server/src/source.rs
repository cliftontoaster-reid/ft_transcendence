use std::path::{Path, PathBuf};

use uuid::Uuid;

/// Layout: `{media_root}/{media_id}/source.*`.
///
/// Returns the first regular file in the media directory, preferring
/// common container extensions. Replace with a `dionysus_db` lookup if
/// the source path should come from the database instead.
pub async fn resolve_source(media_root: &Path, media_id: &Uuid) -> Option<PathBuf> {
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
