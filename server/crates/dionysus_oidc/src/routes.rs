use chrono::{DateTime, Utc};
use rocket::{
  Request, Route, State,
  form::FromForm,
  get,
  http::{Cookie, CookieJar, SameSite, Status},
  post,
  response::{Redirect, Responder, Result as ResponseResult},
  routes,
  serde::json::Json,
};
use serde::Serialize;
use url::Url;
use uuid::Uuid;

use crate::{
  account::ProvisionedAccount,
  error::AuthError,
  guard::{ClientInfo, SessionToken, SessionUser},
  login::CallbackRequest,
  session::SESSION_COOKIE,
  state::AuthState,
};

#[derive(Debug, FromForm)]
pub struct LoginQuery {
  pub redirect_uri: Option<String>,
}

#[derive(Debug, FromForm)]
pub struct CallbackQuery {
  pub code: Option<String>,
  pub state: Option<String>,
  pub error: Option<String>,
  pub error_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
  pub user_id: Uuid,
  pub session_id: Uuid,
  pub active_profile_id: Option<Uuid>,
  pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
  pub account: ProvisionedAccount,
  pub session: SessionResponse,
  pub token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
  pub session: SessionResponse,
  pub token: String,
}

pub enum LoginResponder {
  Redirect(Redirect),
  Json(Json<LoginResponse>),
}

impl<'r> Responder<'r, 'static> for LoginResponder {
  fn respond_to(self, request: &'r Request<'_>) -> ResponseResult<'static> {
    match self {
      Self::Redirect(redirect) => redirect
        .respond_to(request)
        .map_err(|_| Status::InternalServerError),
      Self::Json(json) => json
        .respond_to(request)
        .map_err(|_| Status::InternalServerError),
    }
  }
}

pub enum LogoutResponder {
  Redirect(Box<Redirect>),
  NoContent,
}

impl<'r> Responder<'r, 'static> for LogoutResponder {
  fn respond_to(self, request: &'r Request<'_>) -> ResponseResult<'static> {
    match self {
      Self::Redirect(redirect) => redirect
        .respond_to(request)
        .map_err(|_| Status::InternalServerError),
      Self::NoContent => Status::NoContent
        .respond_to(request)
        .map_err(|_| Status::InternalServerError),
    }
  }
}

#[get("/auth/providers")]
pub fn list_providers(state: &State<AuthState>) -> Json<Vec<String>> {
  let mut providers: Vec<String> = state.login.config().provider.keys().cloned().collect();
  providers.sort();
  Json(providers)
}

#[get("/auth/login?<query..>")]
pub async fn login_default(
  query: LoginQuery,
  state: &State<AuthState>,
) -> Result<Redirect, AuthError> {
  let provider = state.login.config().default.clone();
  authorization_redirect(&provider, query, state).await
}

#[get("/auth/login/<provider>?<query..>")]
pub async fn login(
  provider: &str,
  query: LoginQuery,
  state: &State<AuthState>,
) -> Result<Redirect, AuthError> {
  authorization_redirect(provider, query, state).await
}

async fn authorization_redirect(
  provider: &str,
  query: LoginQuery,
  state: &State<AuthState>,
) -> Result<Redirect, AuthError> {
  let requested_redirect_uri = query
    .redirect_uri
    .as_deref()
    .map(Url::parse)
    .transpose()
    .map_err(|error| AuthError::InvalidRequest(error.to_string()))?;

  let request = state
    .login
    .authorization_url(provider, requested_redirect_uri.as_ref())
    .await?;

  Ok(Redirect::to(request.authorization_url.to_string()))
}

#[get("/auth/callback?<query..>")]
pub async fn callback(
  query: CallbackQuery,
  cookies: &CookieJar<'_>,
  client: ClientInfo,
  state: &State<AuthState>,
) -> Result<LoginResponder, AuthError> {
  let identity = state
    .login
    .handle_callback(CallbackRequest {
      code: query.code,
      state: query.state,
      error: query.error,
      error_description: query.error_description,
    })
    .await?;

  let account = state.provision(&identity).await?;
  let issued = state
    .issue_session(account.user_id, Some(account.profile_id), client.user_agent)
    .await?;

  set_session_cookie(
    cookies,
    &issued.token,
    issued.expires_at,
    state.sessions.secure_cookie(),
  );

  let session = SessionResponse {
    user_id: account.user_id,
    session_id: issued.session_id,
    active_profile_id: Some(account.profile_id),
    expires_at: issued.expires_at,
  };

  if let Some(redirect) = state.post_login_redirect.as_ref() {
    return Ok(LoginResponder::Redirect(Redirect::to(redirect.to_string())));
  }

  Ok(LoginResponder::Json(Json(LoginResponse {
    account,
    session,
    token: issued.token,
  })))
}

#[get("/auth/session")]
pub fn current_session(user: SessionUser) -> Json<SessionResponse> {
  Json(SessionResponse {
    user_id: user.user_id,
    session_id: user.session_id,
    active_profile_id: user.active_profile_id,
    expires_at: user.expires_at,
  })
}

#[post("/auth/refresh")]
pub async fn refresh(
  user: SessionUser,
  cookies: &CookieJar<'_>,
  client: ClientInfo,
  state: &State<AuthState>,
) -> Result<Json<RefreshResponse>, AuthError> {
  state.revoke_session(user.jti).await?;

  let issued = state
    .issue_session(user.user_id, user.active_profile_id, client.user_agent)
    .await?;

  set_session_cookie(
    cookies,
    &issued.token,
    issued.expires_at,
    state.sessions.secure_cookie(),
  );

  let session = SessionResponse {
    user_id: user.user_id,
    session_id: issued.session_id,
    active_profile_id: user.active_profile_id,
    expires_at: issued.expires_at,
  };

  Ok(Json(RefreshResponse {
    session,
    token: issued.token,
  }))
}

#[post("/auth/logout")]
pub async fn logout(
  token: SessionToken,
  cookies: &CookieJar<'_>,
  state: &State<AuthState>,
) -> Result<LogoutResponder, AuthError> {
  cookies.remove(Cookie::from(SESSION_COOKIE));

  if let Some(token) = token.0
    && let Ok(session) = state.verify_session(token).await
  {
    state.revoke_session(session.jti).await?;
  }

  if let Some(redirect) = state.post_logout_redirect.as_ref() {
    return Ok(LogoutResponder::Redirect(Box::new(Redirect::to(
      redirect.to_string(),
    ))));
  }

  Ok(LogoutResponder::NoContent)
}

#[post("/auth/logout/all")]
pub async fn logout_all(
  user: SessionUser,
  cookies: &CookieJar<'_>,
  state: &State<AuthState>,
) -> Result<Status, AuthError> {
  state.revoke_all_sessions(user.user_id).await?;
  cookies.remove(Cookie::from(SESSION_COOKIE));

  Ok(Status::NoContent)
}

pub fn auth_routes() -> Vec<Route> {
  routes![
    list_providers,
    login_default,
    login,
    callback,
    current_session,
    refresh,
    logout,
    logout_all,
  ]
}

fn set_session_cookie(
  cookies: &CookieJar<'_>,
  token: &str,
  expires_at: DateTime<Utc>,
  secure: bool,
) {
  let expires = rocket::time::OffsetDateTime::from_unix_timestamp(expires_at.timestamp())
    .unwrap_or_else(|_| rocket::time::OffsetDateTime::now_utc());

  cookies.add(
    Cookie::build((SESSION_COOKIE, token.to_owned()))
      .path("/")
      .http_only(true)
      .secure(secure)
      .same_site(SameSite::Lax)
      .expires(expires)
      .build(),
  );
}
