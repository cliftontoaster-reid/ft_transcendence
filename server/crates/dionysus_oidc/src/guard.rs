use std::convert::Infallible;

use chrono::{DateTime, Utc};
use rocket::{
  Request, State,
  http::Status,
  request::{FromRequest, Outcome},
};
use uuid::Uuid;

use crate::{error::AuthError, session::SESSION_COOKIE, state::AuthState};

/// An authenticated request, resolved from the session cookie or a bearer
/// token, and validated against the `sessions` table.
#[derive(Debug, Clone)]
pub struct SessionUser {
  pub user_id: Uuid,
  pub session_id: Uuid,
  pub jti: Uuid,
  pub active_profile_id: Option<Uuid>,
  pub expires_at: DateTime<Utc>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SessionUser {
  type Error = AuthError;

  async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
    let state = match request.guard::<&State<AuthState>>().await {
      Outcome::Success(state) => state,
      Outcome::Error(_) => {
        return Outcome::Error((Status::InternalServerError, AuthError::Unavailable));
      }
      Outcome::Forward(forward) => return Outcome::Forward(forward),
    };

    let Some(token) = extract_token(request) else {
      return Outcome::Error((Status::Unauthorized, AuthError::MissingSession));
    };

    match state.verify_session(token).await {
      Ok(session) => Outcome::Success(Self {
        user_id: session.user_id,
        session_id: session.session_id,
        jti: session.jti,
        active_profile_id: session.active_profile_id,
        expires_at: session.expires_at,
      }),
      Err(error) => Outcome::Error((error.status(), error)),
    }
  }
}

/// Best-effort request metadata attached to a session row.
#[derive(Debug, Clone, Default)]
pub struct ClientInfo {
  pub user_agent: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientInfo {
  type Error = Infallible;

  async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Infallible> {
    Outcome::Success(Self {
      user_agent: request.headers().get_one("User-Agent").map(str::to_owned),
    })
  }
}

/// The raw session token, when present, without requiring it to be valid.
/// Used by logout so an expired or revoked token can still be cleaned up.
#[derive(Debug, Clone)]
pub struct SessionToken(pub Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SessionToken {
  type Error = Infallible;

  async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Infallible> {
    Outcome::Success(Self(extract_token(request)))
  }
}

/// `Authorization: Bearer <token>` takes precedence over the cookie.
pub fn extract_token(request: &Request<'_>) -> Option<String> {
  if let Some(header) = request.headers().get_one("Authorization")
    && let Some(token) = header.strip_prefix("Bearer ")
    && !token.trim().is_empty()
  {
    return Some(token.trim().to_string());
  }

  request
    .cookies()
    .get(SESSION_COOKIE)
    .map(|cookie| cookie.value().to_owned())
}
