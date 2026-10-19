use std::path::PathBuf;

use clap::{Parser, Subcommand};
use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::Json;

use dionysus_media_importer::config::ImportConfig;
use dionysus_media_importer::pipeline::{Importer, ScanSummary};

#[derive(Parser)]
#[command(name = "dionysus-importer", about = "Lossless media import pipeline")]
struct Cli {
  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  /// Import every video container under <dir> (default: $IMPORT_DIR).
  Scan { dir: Option<PathBuf> },
  /// Serve POST /import/scan, scanning $IMPORT_DIR on demand.
  Serve,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let _ = tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .try_init();

  let cli = Cli::parse();
  match cli.command {
    Command::Scan { dir } => {
      let config = ImportConfig::from_env();
      let dir = dir.unwrap_or_else(|| config.import_dir.clone());
      let mut importer = Importer::connect(&config).await?;
      let summary = importer.import_directory(&dir).await;
      println!("{}", serde_json::to_string_pretty(&summary)?);
      if !summary.failed.is_empty() {
        std::process::exit(1);
      }
    }
    Command::Serve => {
      let rocket = rocket::build()
        .manage(ImportConfig::from_env())
        .mount("/", rocket::routes![import_scan]);
      rocket.launch().await?;
    }
  }
  Ok(())
}

#[rocket::post("/import/scan")]
async fn import_scan(
  config: &rocket::State<ImportConfig>,
) -> Result<Json<ScanSummary>, Custom<String>> {
  let mut importer = Importer::connect(config.inner())
    .await
    .map_err(|e| Custom(Status::InternalServerError, e.to_string()))?;
  let summary = importer.import_directory(&config.import_dir).await;
  Ok(Json(summary))
}
