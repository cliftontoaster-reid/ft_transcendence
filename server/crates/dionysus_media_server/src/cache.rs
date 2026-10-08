use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Freshness check for a cached file: exists and younger than `ttl_secs`.
pub async fn read_if_fresh(path: &Path, ttl_secs: u64) -> Option<Vec<u8>> {
  let meta = tokio::fs::metadata(path).await.ok()?;
  if !meta.is_file() {
    return None;
  }
  let age = meta
    .modified()
    .ok()
    .and_then(|m| SystemTime::now().duration_since(m).ok())
    .map(|d| d.as_secs())
    .unwrap_or(u64::MAX);
  if age > ttl_secs {
    return None;
  }
  tokio::fs::read(path).await.ok()
}

/// Atomically store `bytes` at `path` (write tmp + rename), creating parents.
pub async fn store(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
  if let Some(parent) = path.parent() {
    tokio::fs::create_dir_all(parent).await?;
  }
  let tmp = path.with_extension("tmp");
  tokio::fs::write(&tmp, bytes).await?;
  tokio::fs::rename(&tmp, path).await?;
  Ok(())
}

/// Delete oldest-first (by mtime) until the tree under `cache_dir` fits
/// in `max_bytes`. Best-effort: I/O errors are ignored.
pub async fn enforce_size_limit(cache_dir: &Path, max_bytes: u64) {
  let entries = collect_entries(cache_dir).await;
  let total: u64 = entries.iter().map(|(_, _, size)| size).sum();
  if total <= max_bytes {
    return;
  }
  let mut entries = entries;
  // Oldest first.
  entries.sort_by_key(|(_, mtime, _)| *mtime);
  let mut running = total;
  for (path, _, size) in entries {
    if running <= max_bytes {
      break;
    }
    if tokio::fs::remove_file(&path).await.is_ok() {
      running = running.saturating_sub(size);
    }
  }
}

type Entry = (PathBuf, SystemTime, u64);

async fn collect_entries(dir: &Path) -> Vec<Entry> {
  let mut out = Vec::new();
  let mut stack = vec![dir.to_path_buf()];
  while let Some(d) = stack.pop() {
    let mut rd = match tokio::fs::read_dir(&d).await {
      Ok(rd) => rd,
      Err(_) => continue,
    };
    while let Ok(Some(e)) = rd.next_entry().await {
      let p = e.path();
      let Ok(ft) = e.file_type().await else {
        continue;
      };
      if ft.is_dir() {
        stack.push(p);
      } else if ft.is_file() {
        // Skip in-flight writes.
        if p.extension().and_then(|x| x.to_str()) == Some("tmp") {
          continue;
        }
        let Ok(meta) = e.metadata().await else {
          continue;
        };
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        out.push((p, mtime, meta.len()));
      }
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn size_limit_evicts_oldest_first() {
    let dir = std::env::temp_dir().join(format!("dionysus_cache_test_{}", std::process::id()));
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();

    // Create oldest -> newest so mtimes are ordered (fs granularity is ns).
    for name in ["old.fmp4", "mid.fmp4", "new.fmp4"] {
      tokio::fs::write(dir.join(name), vec![0u8; 10]).await.unwrap();
      tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // 3 x 10 bytes with a 25-byte cap must evict exactly the oldest file.
    enforce_size_limit(&dir, 25).await;
    assert!(tokio::fs::metadata(dir.join("old.fmp4")).await.is_err());
    assert!(tokio::fs::metadata(dir.join("mid.fmp4")).await.is_ok());
    assert!(tokio::fs::metadata(dir.join("new.fmp4")).await.is_ok());
    let _ = tokio::fs::remove_dir_all(&dir).await;
  }
}
