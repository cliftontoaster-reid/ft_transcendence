use diesel::{Connection, PgConnection};
use url::Url;
use uuid::Uuid;

use crate::{
  account::{self, ProvisionedAccount},
  error::AuthError,
  login::{AuthenticatedUser, LoginUrlError, LoginUrlService},
  session::{IssuedSession, SessionService, VerifiedSession},
};

/// Shared, clonable application state for the auth routes and request guards.
pub struct AuthState {
  pub login: LoginUrlService,
  pub sessions: SessionService,
  pub database_url: String,
  pub post_login_redirect: Option<Url>,
  pub post_logout_redirect: Option<Url>,
}

impl AuthState {
  /// Run a synchronous Diesel operation on Rocket's blocking pool. A fresh
  /// connection is opened per task, which keeps `AuthState` `Send + Sync`
  /// without introducing a connection-pool dependency.
  pub async fn with_connection<T, E, F>(&self, operation: F) -> Result<T, AuthError>
  where
    F: FnOnce(&mut PgConnection) -> Result<T, E> + Send + 'static,
    E: Into<AuthError> + Send + 'static,
    T: Send + 'static,
  {
    let database_url = self.database_url.clone();

    tokio::task::spawn_blocking(move || {
      let mut connection = PgConnection::establish(&database_url)
        .map_err(|error| AuthError::Connection(error.to_string()))?;
      operation(&mut connection).map_err(Into::into)
    })
    .await?
  }

  pub async fn provision(
    &self,
    identity: &AuthenticatedUser,
  ) -> Result<ProvisionedAccount, AuthError> {
    let provider = self
      .login
      .provider_config(&identity.provider)
      .ok_or_else(|| LoginUrlError::UnknownProvider(identity.provider.clone()))?
      .clone();
    let identity = identity.clone();

    self
      .with_connection(move |connection| {
        account::provision_account(connection, &identity, &provider)
      })
      .await
  }

  pub async fn issue_session(
    &self,
    user_id: Uuid,
    active_profile_id: Option<Uuid>,
    user_agent: Option<String>,
  ) -> Result<IssuedSession, AuthError> {
    let sessions = self.sessions.clone();

    self
      .with_connection(move |connection| {
        sessions.issue(
          connection,
          user_id,
          active_profile_id,
          user_agent.as_deref(),
        )
      })
      .await
  }

  pub async fn verify_session(&self, token: String) -> Result<VerifiedSession, AuthError> {
    let sessions = self.sessions.clone();

    self
      .with_connection(move |connection| sessions.verify(connection, &token))
      .await
  }

  pub async fn revoke_session(&self, jti: Uuid) -> Result<(), AuthError> {
    let sessions = self.sessions.clone();

    self
      .with_connection(move |connection| sessions.revoke(connection, jti))
      .await
  }

  pub async fn revoke_all_sessions(&self, user_id: Uuid) -> Result<usize, AuthError> {
    let sessions = self.sessions.clone();

    self
      .with_connection(move |connection| sessions.revoke_all(connection, user_id))
      .await
  }
}
