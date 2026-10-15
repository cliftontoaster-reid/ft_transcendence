use diesel::{
  prelude::*,
  result::{DatabaseErrorKind, Error as DieselError},
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenvy::dotenv;
use std::{env, error::Error};
use uuid::Uuid;

pub mod models;
pub mod schema;
pub mod types;

use schema::users;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations");

pub fn establish_connection() -> PgConnection {
  dotenv().ok();

  let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
  PgConnection::establish(&database_url)
    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn run_migrations(
  conn: &mut PgConnection,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
  conn.run_pending_migrations(MIGRATIONS)?;
  Ok(())
}

pub fn find_user(conn: &mut PgConnection, user_id: Uuid) -> QueryResult<Option<models::User>> {
  users::table
    .find(user_id)
    .select(models::User::as_select())
    .first(conn)
    .optional()
}

pub fn update_user_profile(
  conn: &mut PgConnection,
  user_id: Uuid,
  changes: models::UserProfileChanges<'_>,
) -> QueryResult<models::User> {
  diesel::update(users::table.find(user_id))
    .set(changes)
    .returning(models::User::as_returning())
    .get_result(conn)
}

pub fn validate_profile(
  username: Option<&str>,
  display_name: Option<&str>,
  bio: Option<&str>,
  avatar_url: Option<&str>,
  age_bracket: Option<i32>,
) -> Result<(), &'static str> {
  if username.is_some_and(|value| value.is_empty() || value.len() > 64) {
    return Err("username must contain between 1 and 64 characters");
  }
  if display_name.is_some_and(|value| value.trim().is_empty() || value.len() > 256) {
    return Err("display_name must contain between 1 and 256 characters");
  }
  if bio.is_some_and(|value| value.len() > 500) {
    return Err("bio must contain at most 500 characters");
  }
  if avatar_url.is_some_and(|value| value.len() > 2048) {
    return Err("avatar_url must contain at most 2048 characters");
  }
  if age_bracket.is_some_and(|value| !(0..=5).contains(&value)) {
    return Err("age_bracket must be between 0 and 5");
  }
  Ok(())
}

pub fn is_username_conflict(error: &DieselError) -> bool {
  matches!(
    error,
    DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)
  )
}
