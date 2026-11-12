use std::env;

use rocket::{Build, Rocket};
use thiserror::Error;

pub mod account;
pub mod error;
pub mod guard;
pub mod login;
pub mod model;
pub mod routes;
pub mod session;
pub mod state;

use crate::{
  login::LoginUrlService,
  model::config::OidcConfig,
  session::{SessionConfig, SessionService},
  state::AuthState,
};

#[derive(Debug, Error)]
pub enum StartupError {
  #[error("DATABASE_URL must be set")]
  MissingDatabaseUrl,

  #[error("invalid REDIS_URL: {0}")]
  Redis(#[from] redis::RedisError),

  #[error(transparent)]
  SessionConfig(#[from] session::SessionConfigError),

  #[error("invalid session configuration: {0}")]
  SessionInit(String),
}

/// Build the OIDC auth service. The returned Rocket instance can be launched
/// directly (see `main.rs`) or mounted into a larger application.
pub fn build_rocket(config: OidcConfig) -> Result<Rocket<Build>, StartupError> {
  let database_url = env::var("DATABASE_URL").map_err(|_| StartupError::MissingDatabaseUrl)?;
  let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
  let redis = redis::Client::open(redis_url)?;

  let sessions = SessionService::new(SessionConfig::from_env()?)
    .map_err(|error| StartupError::SessionInit(error.to_string()))?;

  let state = AuthState {
    login: LoginUrlService::new(config.clone(), redis),
    sessions,
    database_url,
    post_login_redirect: config.post_login_redirect,
    post_logout_redirect: config.post_logout_redirect,
  };

  Ok(
    rocket::build()
      .manage(state)
      .mount("/", routes::auth_routes()),
  )
}
