use std::path::PathBuf;

/// Runtime configuration, all from env with sane defaults.
#[derive(Debug, Clone)]
pub struct ImportConfig {
  /// Where imported media lives: one `{uuid}/` directory per item.
  pub media_root: PathBuf,
  /// Default directory scanned by `serve` (the `scan` subcommand takes an
  /// explicit path argument that overrides this).
  pub import_dir: PathBuf,
  /// Full gRPC endpoint of the encoder, e.g. `http://[::1]:50051`.
  pub encoder_addr: String,
}

fn env_str(key: &str, default: &str) -> String {
  std::env::var(key).unwrap_or_else(|_| default.to_string())
}

impl ImportConfig {
  pub fn from_env() -> Self {
    Self {
      media_root: PathBuf::from(env_str("MEDIA_ROOT", "./media")),
      import_dir: PathBuf::from(env_str("IMPORT_DIR", "./import")),
      encoder_addr: env_str("ENCODER_ADDR", "http://[::1]:50051"),
    }
  }
}
