//! Walks an import directory and content-hashes candidates for dedup.

use std::io::Read;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Container extensions the scanner will import.
pub const VIDEO_CONTAINER_EXTENSIONS: &[&str] = &["mkv", "mp4", "mov", "ts", "m4v", "webm"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
  pub path: PathBuf,
  pub size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
  #[error("cannot scan {dir}: {source}")]
  Walk {
    dir: PathBuf,
    source: walkdir::Error,
  },
  #[error("i/o: {0}")]
  Io(#[from] std::io::Error),
}

/// Every video container under `dir`, sorted for deterministic import order.
/// Hidden entries (dot files/dirs) are skipped; symlinks are not followed.
/// Unreadable individual entries are logged and skipped — one broken symlink
/// must not abort the scan.
pub fn scan(dir: &Path) -> Result<Vec<ScannedFile>, ScanError> {
  let mut files = Vec::new();
  let entries = WalkDir::new(dir).into_iter().filter_entry(|e| {
    // The scan root may legitimately live in a dot dir (e.g. /tmp/.tmpXYZ).
    e.depth() == 0
      || !e
        .file_name()
        .to_str()
        .is_some_and(|name| name.starts_with('.'))
  });
  for entry in entries {
    let entry = match entry {
      Ok(entry) => entry,
      Err(err) => {
        // The root itself failing means the directory is unreadable — that's
        // an error; deeper failures (broken symlinks) just skip the entry.
        if err.depth() == 0 {
          return Err(ScanError::Walk {
            dir: dir.to_path_buf(),
            source: err,
          });
        }
        tracing::warn!(%err, "skipping unreadable entry during scan");
        continue;
      }
    };
    if !entry.file_type().is_file() {
      continue;
    }
    let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) else {
      continue;
    };
    if !VIDEO_CONTAINER_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
      continue;
    }
    files.push(ScannedFile {
      path: entry.path().to_path_buf(),
      size: entry.metadata().map(|m| m.len()).unwrap_or(0),
    });
  }
  files.sort_by(|a, b| a.path.cmp(&b.path));
  Ok(files)
}

/// blake3 content hash (hex) — the dedup key stored in `media.source_hash`.
pub fn hash_file(path: &Path) -> Result<String, ScanError> {
  let mut hasher = blake3::Hasher::new();
  let mut file = std::io::BufReader::new(std::fs::File::open(path)?);
  let mut buf = vec![0u8; 64 * 1024];
  loop {
    let read = file.read(&mut buf)?;
    if read == 0 {
      break;
    }
    hasher.update(&buf[..read]);
  }
  Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn scan_filters_by_extension_and_skips_hidden() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("a.mkv"), b"x").unwrap();
    std::fs::write(dir.path().join("B.MP4"), b"x").unwrap();
    std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub").join("c.mov"), b"x").unwrap();
    std::fs::create_dir(dir.path().join(".hidden")).unwrap();
    std::fs::write(dir.path().join(".hidden").join("d.ts"), b"x").unwrap();

    let files = scan(dir.path()).unwrap();
    let names: Vec<&str> = files
      .iter()
      .map(|f| f.path.file_name().unwrap().to_str().unwrap())
      .collect();
    assert_eq!(names, ["B.MP4", "a.mkv", "c.mov"]); // sorted by path
  }

  #[test]
  fn hash_matches_blake3_of_contents() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("file.mkv");
    std::fs::write(&path, b"transcendence").unwrap();
    assert_eq!(
      hash_file(&path).unwrap(),
      blake3::hash(b"transcendence").to_hex().to_string()
    );
  }

  #[test]
  fn scan_of_missing_dir_is_an_error() {
    assert!(scan(Path::new("/nonexistent/dionysus/scan")).is_err());
  }
}
