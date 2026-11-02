use std::{path::PathBuf, process::ExitStatus};

/// Failure while probing a media file.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
  #[error("no such media file: {0}")]
  NotFound(PathBuf),
  #[error("not a regular file: {0}")]
  NotAFile(PathBuf),
  #[error("ffprobe executable not found; install ffmpeg")]
  MissingExecutable,
  #[error("failed to run ffprobe: {0}")]
  Spawn(#[source] std::io::Error),
  #[error("ffprobe failed ({}): {stderr}", status_exit(.status))]
  Failed { status: ExitStatus, stderr: String },
  #[error("cannot parse ffprobe output: {0}")]
  Parse(#[from] serde_json::Error),
}

fn status_exit(status: &ExitStatus) -> String {
  status
    .code()
    .map_or_else(|| "signal".to_string(), |c| format!("exit {c}"))
}
