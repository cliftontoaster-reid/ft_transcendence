use diesel::prelude::*;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenvy::dotenv;
use std::{env, error::Error};

pub mod models;
pub mod schema;
pub mod types;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations");

pub fn establish_connection() -> PgConnection {
  dotenv().ok();

  let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
  PgConnection::establish(&database_url)
    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

/// Non-panicking [`establish_connection`]: `None` when `DATABASE_URL` is
/// unset or the connection fails, so services can degrade to filesystem
/// fallbacks instead of refusing to start.
pub fn try_establish_connection() -> Option<PgConnection> {
  dotenv().ok();

  let database_url = env::var("DATABASE_URL").ok()?;
  PgConnection::establish(&database_url).ok()
}

pub fn run_migrations(
  conn: &mut PgConnection,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
  conn.run_pending_migrations(MIGRATIONS)?;
  Ok(())
}
