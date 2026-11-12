use std::env;

use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use dionysus_db::schema::{sessions, users};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const SESSION_COOKIE: &str = "dionysus_session";

const TOKEN_ISSUER: &str = "dionysus";
const MIN_SECRET_LEN: usize = 32;
const DEFAULT_TTL_SECONDS: i64 = 60 * 60 * 24;

#[derive(Debug, Error)]
pub enum SessionConfigError {
  #[error("SESSION_SECRET must be set")]
  MissingSecret,

  #[error("SESSION_SECRET must be at least {MIN_SECRET_LEN} bytes")]
  WeakSecret,

  #[error("SESSION_TTL_SECONDS must be a positive integer")]
  InvalidTtl,
}

#[derive(Debug, Error)]
pub enum SessionError {
  #[error("database error: {0}")]
  Database(#[from] diesel::result::Error),

  #[error("invalid session token: {0}")]
  Token(#[from] jsonwebtoken::errors::Error),

  #[error("session secret is too short")]
  WeakSecret,

  #[error("session not found")]
  NotFound,

  #[error("session has been revoked")]
  Revoked,

  #[error("session has expired")]
  Expired,

  #[error("user account is disabled")]
  Disabled,
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
  pub secret: Vec<u8>,
  pub ttl: Duration,
  pub secure_cookie: bool,
}

impl SessionConfig {
  pub fn from_env() -> Result<Self, SessionConfigError> {
    let secret = env::var("SESSION_SECRET").map_err(|_| SessionConfigError::MissingSecret)?;
    let ttl_seconds = match env::var("SESSION_TTL_SECONDS") {
      Ok(value) => value
        .parse::<i64>()
        .ok()
        .filter(|seconds| *seconds > 0)
        .ok_or(SessionConfigError::InvalidTtl)?,
      Err(_) => DEFAULT_TTL_SECONDS,
    };
    let secure_cookie = env::var("SESSION_COOKIE_SECURE")
      .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes"))
      .unwrap_or(true);

    let config = Self {
      secret: secret.into_bytes(),
      ttl: Duration::seconds(ttl_seconds),
      secure_cookie,
    };
    config.validate()?;
    Ok(config)
  }

  fn validate(&self) -> Result<(), SessionConfigError> {
    if self.secret.len() < MIN_SECRET_LEN {
      return Err(SessionConfigError::WeakSecret);
    }
    if self.ttl <= Duration::zero() {
      return Err(SessionConfigError::InvalidTtl);
    }
    Ok(())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionClaims {
  pub sub: Uuid,
  pub jti: Uuid,
  pub iat: i64,
  pub exp: i64,
  pub iss: String,
}

#[derive(Debug, Clone)]
pub struct IssuedSession {
  pub session_id: Uuid,
  pub token: String,
  pub jti: Uuid,
  pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct VerifiedSession {
  pub session_id: Uuid,
  pub user_id: Uuid,
  pub jti: Uuid,
  pub active_profile_id: Option<Uuid>,
  pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionService {
  secret: Vec<u8>,
  ttl: Duration,
  secure_cookie: bool,
}

/// The subset of a `sessions` row needed to validate a token.
type SessionRow = (
  Uuid,
  Uuid,
  Option<Uuid>,
  DateTime<Utc>,
  Option<DateTime<Utc>>,
);

impl SessionService {
  pub fn new(config: SessionConfig) -> Result<Self, SessionError> {
    config.validate().map_err(|_| SessionError::WeakSecret)?;

    Ok(Self {
      secret: config.secret,
      ttl: config.ttl,
      secure_cookie: config.secure_cookie,
    })
  }

  pub fn ttl(&self) -> Duration {
    self.ttl
  }

  pub fn secure_cookie(&self) -> bool {
    self.secure_cookie
  }

  pub fn issue(
    &self,
    conn: &mut PgConnection,
    user_id: Uuid,
    active_profile_id: Option<Uuid>,
    user_agent: Option<&str>,
  ) -> Result<IssuedSession, SessionError> {
    let now = Utc::now();
    let expires_at = now + self.ttl;
    let jti = Uuid::new_v4();

    let session_id: Uuid = diesel::insert_into(sessions::table)
      .values(NewSessionRow {
        user_id,
        active_profile_id,
        jti,
        expires_at,
        user_agent,
      })
      .returning(sessions::id)
      .get_result(conn)?;

    let claims = SessionClaims {
      sub: user_id,
      jti,
      iat: now.timestamp(),
      exp: expires_at.timestamp(),
      iss: TOKEN_ISSUER.to_string(),
    };
    let token = encode(
      &Header::new(Algorithm::HS256),
      &claims,
      &EncodingKey::from_secret(&self.secret),
    )?;

    Ok(IssuedSession {
      session_id,
      token,
      jti,
      expires_at,
    })
  }

  pub fn verify(
    &self,
    conn: &mut PgConnection,
    token: &str,
  ) -> Result<VerifiedSession, SessionError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[TOKEN_ISSUER]);
    validation.set_required_spec_claims(&["exp", "sub", "iss"]);

    let claims =
      decode::<SessionClaims>(token, &DecodingKey::from_secret(&self.secret), &validation)?.claims;

    let row: Option<SessionRow> = sessions::table
      .filter(sessions::jti.eq(claims.jti))
      .select((
        sessions::id,
        sessions::user_id,
        sessions::active_profile_id,
        sessions::expires_at,
        sessions::revoked_at,
      ))
      .first(conn)
      .optional()?;

    let Some((session_id, user_id, active_profile_id, expires_at, revoked_at)) = row else {
      return Err(SessionError::NotFound);
    };

    if user_id != claims.sub {
      return Err(SessionError::NotFound);
    }
    if revoked_at.is_some() {
      return Err(SessionError::Revoked);
    }
    if expires_at <= Utc::now() {
      return Err(SessionError::Expired);
    }

    let disabled: bool = users::table
      .find(user_id)
      .select(users::disabled)
      .first(conn)?;
    if disabled {
      return Err(SessionError::Disabled);
    }

    diesel::update(sessions::table.filter(sessions::jti.eq(claims.jti)))
      .set(sessions::last_seen_at.eq(Utc::now()))
      .execute(conn)?;

    Ok(VerifiedSession {
      session_id,
      user_id,
      jti: claims.jti,
      active_profile_id,
      expires_at,
    })
  }

  pub fn revoke(&self, conn: &mut PgConnection, jti: Uuid) -> Result<(), SessionError> {
    diesel::update(sessions::table.filter(sessions::jti.eq(jti)))
      .set(sessions::revoked_at.eq(Utc::now()))
      .execute(conn)?;
    Ok(())
  }

  pub fn revoke_all(&self, conn: &mut PgConnection, user_id: Uuid) -> Result<usize, SessionError> {
    diesel::update(
      sessions::table
        .filter(sessions::user_id.eq(user_id))
        .filter(sessions::revoked_at.is_null()),
    )
    .set(sessions::revoked_at.eq(Utc::now()))
    .execute(conn)
    .map_err(Into::into)
  }
}

#[derive(Insertable)]
#[diesel(table_name = sessions)]
struct NewSessionRow<'a> {
  user_id: Uuid,
  active_profile_id: Option<Uuid>,
  jti: Uuid,
  expires_at: DateTime<Utc>,
  user_agent: Option<&'a str>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_short_secrets() {
    let service = SessionService::new(SessionConfig {
      secret: b"too-short".to_vec(),
      ttl: Duration::hours(1),
      secure_cookie: true,
    });

    assert!(matches!(service, Err(SessionError::WeakSecret)));
  }

  #[test]
  fn accepts_long_secrets() {
    let service = SessionService::new(SessionConfig {
      secret: b"a-secret-that-is-at-least-32-bytes!".to_vec(),
      ttl: Duration::hours(1),
      secure_cookie: false,
    })
    .unwrap();

    assert_eq!(service.ttl(), Duration::hours(1));
    assert!(!service.secure_cookie());
  }
}
