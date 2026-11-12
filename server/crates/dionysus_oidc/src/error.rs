use rocket::{
  Request,
  http::Status,
  response::{Responder, Result as ResponseResult},
};
use thiserror::Error;

use crate::{account::AccountError, login::LoginUrlError, session::SessionError};

#[derive(Debug, Error)]
pub enum AuthError {
  #[error(transparent)]
  Login(#[from] LoginUrlError),

  #[error(transparent)]
  Account(#[from] AccountError),

  #[error(transparent)]
  Session(#[from] SessionError),

  #[error("database error: {0}")]
  Database(#[from] diesel::result::Error),

  #[error("database connection failed: {0}")]
  Connection(String),

  #[error("background database task failed: {0}")]
  Task(#[from] tokio::task::JoinError),

  #[error("missing session token")]
  MissingSession,

  #[error("invalid request: {0}")]
  InvalidRequest(String),

  #[error("auth service is unavailable")]
  Unavailable,
}

impl AuthError {
  pub fn status(&self) -> Status {
    match self {
      Self::Login(error) => login_status(error),
      Self::Account(error) => account_status(error),
      Self::Session(error) => session_status(error),
      Self::MissingSession => Status::Unauthorized,
      Self::InvalidRequest(_) => Status::BadRequest,
      Self::Database(_) | Self::Task(_) | Self::Unavailable | Self::Connection(_) => {
        Status::InternalServerError
      }
    }
  }
}

fn login_status(error: &LoginUrlError) -> Status {
  match error {
    LoginUrlError::UnknownProvider(_)
    | LoginUrlError::MissingRedirectUri(_)
    | LoginUrlError::AmbiguousRedirectUri(_)
    | LoginUrlError::MissingOpenidScope(_)
    | LoginUrlError::InvalidRedirectUri { .. }
    | LoginUrlError::MissingCallbackState
    | LoginUrlError::MissingAuthorizationCode
    | LoginUrlError::Provider(_)
    | LoginUrlError::MissingLoginTransaction
    | LoginUrlError::StateMismatch
    | LoginUrlError::MissingIdToken
    | LoginUrlError::IdTokenVerification(_) => Status::BadRequest,

    LoginUrlError::Secret(_)
    | LoginUrlError::InvalidIssuer(_)
    | LoginUrlError::Discovery(_)
    | LoginUrlError::Redis(_)
    | LoginUrlError::Serialization(_)
    | LoginUrlError::TokenExchange(_) => Status::InternalServerError,
  }
}

fn account_status(error: &AccountError) -> Status {
  match error {
    AccountError::Disabled(_) => Status::Forbidden,
    AccountError::EmptyDisplayName => Status::BadRequest,
    AccountError::Database(_) | AccountError::MissingUser | AccountError::InvalidRoleName(_) => {
      Status::InternalServerError
    }
  }
}

fn session_status(error: &SessionError) -> Status {
  match error {
    SessionError::Token(_)
    | SessionError::NotFound
    | SessionError::Revoked
    | SessionError::Expired => Status::Unauthorized,
    SessionError::Disabled => Status::Forbidden,
    SessionError::Database(_) | SessionError::WeakSecret => Status::InternalServerError,
  }
}

impl<'r> Responder<'r, 'static> for AuthError {
  fn respond_to(self, request: &'r Request<'_>) -> ResponseResult<'static> {
    let status = self.status();
    let body = rocket::serde::json::Json(serde_json::json!({
      "error": self.to_string(),
    }));

    let mut response = match body.respond_to(request) {
      Ok(response) => response,
      Err(_) => return Err(Status::InternalServerError),
    };
    response.set_status(status);

    Ok(response)
  }
}
