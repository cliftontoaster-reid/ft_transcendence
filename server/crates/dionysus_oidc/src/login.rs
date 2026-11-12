use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use openidconnect::{
  AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
  PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
  core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

use crate::model::config::{OidcConfig, OidcProviderConfig, SecretReferenceError};

const LOGIN_TTL_SECONDS: u64 = 600;
const LOGIN_KEY_PREFIX: &str = "oidc:login:";

#[derive(Debug, Error)]
pub enum LoginUrlError {
  #[error("OIDC provider {0:?} is not configured")]
  UnknownProvider(String),
  #[error("OIDC provider {0:?} has no configured redirect URI")]
  MissingRedirectUri(String),
  #[error("OIDC provider {0:?} has multiple redirect URIs; one must be requested explicitly")]
  AmbiguousRedirectUri(String),
  #[error("OIDC provider {0:?} must request the openid scope")]
  MissingOpenidScope(String),
  #[error("requested redirect URI is not configured for provider {provider:?}: {redirect_uri}")]
  InvalidRedirectUri { provider: String, redirect_uri: Url },
  #[error("invalid provider issuer URL: {0}")]
  InvalidIssuer(#[from] openidconnect::url::ParseError),
  #[error("invalid client secret: {0}")]
  Secret(#[from] SecretReferenceError),
  #[error("OIDC discovery failed: {0}")]
  Discovery(String),
  #[error("Redis error: {0}")]
  Redis(#[from] redis::RedisError),
  #[error("failed to serialize login transaction: {0}")]
  Serialization(#[from] serde_json::Error),
  #[error("missing callback state")]
  MissingCallbackState,
  #[error("missing authorization code")]
  MissingAuthorizationCode,
  #[error("OIDC provider returned an error: {0}")]
  Provider(String),
  #[error("login transaction not found or expired")]
  MissingLoginTransaction,
  #[error("callback state does not match the login transaction")]
  StateMismatch,
  #[error("OIDC token exchange failed: {0}")]
  TokenExchange(String),
  #[error("OIDC ID token is missing")]
  MissingIdToken,
  #[error("OIDC ID token verification failed: {0}")]
  IdTokenVerification(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginTransaction {
  pub provider: String,
  pub state: String,
  pub nonce: String,
  pub code_verifier: String,
  pub redirect_uri: Url,
}

#[derive(Debug)]
pub struct LoginRequest {
  pub authorization_url: Url,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthenticatedUser {
  pub provider: String,
  pub issuer: String,
  pub subject: String,
  pub email: Option<String>,
  /// `None` when the provider did not state whether the email is verified.
  pub email_verified: Option<bool>,
  pub preferred_username: Option<String>,
  pub groups: Vec<String>,
}

impl AuthenticatedUser {
  /// Best-effort human-readable name for provisioning a new account.
  pub fn display_name_hint(&self) -> String {
    self
      .preferred_username
      .as_deref()
      .map(str::trim)
      .filter(|value| !value.is_empty())
      .map(str::to_owned)
      .or_else(|| {
        self
          .email
          .as_deref()
          .and_then(|email| email.split('@').next())
          .map(str::to_owned)
      })
      .unwrap_or_else(|| format!("{}:{}", self.provider, self.subject))
  }

  /// The email may only be trusted for account linking when the provider did
  /// not explicitly mark it as unverified. Oversized values are dropped rather
  /// than overflowing the `users.email` column.
  pub fn trustworthy_email(&self) -> Option<&str> {
    self
      .email
      .as_deref()
      .filter(|email| email.len() <= 320)
      .filter(|_| self.email_verified != Some(false))
  }
}

#[derive(Debug, Clone, Default)]
pub struct CallbackRequest {
  pub code: Option<String>,
  pub state: Option<String>,
  pub error: Option<String>,
  pub error_description: Option<String>,
}

pub struct LoginUrlService {
  config: OidcConfig,
  redis: redis::Client,
}

impl LoginUrlService {
  pub fn new(config: OidcConfig, redis: redis::Client) -> Self {
    Self { config, redis }
  }

  pub fn config(&self) -> &OidcConfig {
    &self.config
  }

  pub fn provider_config(&self, provider_name: &str) -> Option<&OidcProviderConfig> {
    self.config.provider(provider_name)
  }

  pub async fn authorization_url(
    &self,
    provider_name: &str,
    requested_redirect_uri: Option<&Url>,
  ) -> Result<LoginRequest, LoginUrlError> {
    let provider = self
      .config
      .provider
      .get(provider_name)
      .ok_or_else(|| LoginUrlError::UnknownProvider(provider_name.to_owned()))?;
    let redirect_uri = select_redirect_uri(provider_name, provider, requested_redirect_uri)?;
    if !provider.scopes.iter().any(|scope| scope == "openid") {
      return Err(LoginUrlError::MissingOpenidScope(provider_name.to_owned()));
    }
    let client_secret = provider.client_secret.resolve()?;
    let issuer = IssuerUrl::new(provider.issuer_url.to_string())?;
    let http_client = reqwest::ClientBuilder::new()
      .redirect(reqwest::redirect::Policy::none())
      .build()
      .map_err(|error| LoginUrlError::Discovery(error.to_string()))?;
    let provider_metadata = CoreProviderMetadata::discover_async(issuer, &http_client)
      .await
      .map_err(|error| LoginUrlError::Discovery(error.to_string()))?;

    let client = CoreClient::from_provider_metadata(
      provider_metadata,
      ClientId::new(provider.client_id.clone()),
      Some(ClientSecret::new(client_secret)),
    )
    .set_redirect_uri(
      RedirectUrl::new(redirect_uri.to_string()).expect("redirect URI was validated"),
    );
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (authorization_url, csrf_state, nonce) = client
      .authorize_url(
        CoreAuthenticationFlow::AuthorizationCode,
        CsrfToken::new_random,
        Nonce::new_random,
      )
      .set_pkce_challenge(pkce_challenge)
      .add_scopes(provider.scopes.iter().cloned().map(Scope::new))
      .url();

    let state = csrf_state.secret().to_owned();
    let transaction = LoginTransaction {
      provider: provider_name.to_owned(),
      state: state.clone(),
      nonce: nonce.secret().to_owned(),
      code_verifier: pkce_verifier.secret().to_owned(),
      redirect_uri,
    };

    store_login_transaction(&self.redis, &transaction).await?;

    Ok(LoginRequest { authorization_url })
  }

  pub async fn handle_callback(
    &self,
    callback: CallbackRequest,
  ) -> Result<AuthenticatedUser, LoginUrlError> {
    let state = callback
      .state
      .filter(|state| !state.is_empty())
      .ok_or(LoginUrlError::MissingCallbackState)?;
    let transaction = consume_login_transaction(&self.redis, &state).await?;

    if transaction.state != state {
      return Err(LoginUrlError::StateMismatch);
    }

    if let Some(error) = callback.error {
      let description = callback.error_description.unwrap_or_default();
      let detail = if description.is_empty() {
        error
      } else {
        format!("{error}: {description}")
      };
      return Err(LoginUrlError::Provider(detail));
    }

    let code = callback
      .code
      .filter(|code| !code.is_empty())
      .ok_or(LoginUrlError::MissingAuthorizationCode)?;

    let provider = self
      .config
      .provider
      .get(&transaction.provider)
      .ok_or_else(|| LoginUrlError::UnknownProvider(transaction.provider.clone()))?;
    let client_secret = provider.client_secret.resolve()?;
    let issuer = IssuerUrl::new(provider.issuer_url.to_string())?;
    let http_client = reqwest::ClientBuilder::new()
      .redirect(reqwest::redirect::Policy::none())
      .build()
      .map_err(|error| LoginUrlError::Discovery(error.to_string()))?;
    let provider_metadata = CoreProviderMetadata::discover_async(issuer, &http_client)
      .await
      .map_err(|error| LoginUrlError::Discovery(error.to_string()))?;
    let client = CoreClient::from_provider_metadata(
      provider_metadata,
      ClientId::new(provider.client_id.clone()),
      Some(ClientSecret::new(client_secret)),
    )
    .set_redirect_uri(
      RedirectUrl::new(transaction.redirect_uri.to_string())
        .expect("redirect URI was validated before storing the transaction"),
    );

    let token_response = client
      .exchange_code(AuthorizationCode::new(code))
      .map_err(|error| LoginUrlError::TokenExchange(error.to_string()))?
      .set_pkce_verifier(PkceCodeVerifier::new(transaction.code_verifier))
      .request_async(&http_client)
      .await
      .map_err(|error| LoginUrlError::TokenExchange(error.to_string()))?;
    let id_token = token_response
      .id_token()
      .ok_or(LoginUrlError::MissingIdToken)?;
    // The signature is verified by `claims()` below. Reading the same raw token
    // afterwards is safe and lets us surface non-standard claims such as the
    // provider's group list without defining a custom OIDC client type.
    let raw_id_token = id_token.to_string();
    let expected_nonce = Nonce::new(transaction.nonce);
    let claims = id_token
      .claims(&client.id_token_verifier(), &expected_nonce)
      .map_err(|error| LoginUrlError::IdTokenVerification(error.to_string()))?;

    Ok(AuthenticatedUser {
      provider: transaction.provider,
      issuer: claims.issuer().as_str().to_owned(),
      subject: claims.subject().as_str().to_owned(),
      email: claims.email().map(|email| email.as_str().to_owned()),
      email_verified: claims.email_verified(),
      preferred_username: claims
        .preferred_username()
        .map(|name| name.as_str().to_owned()),
      groups: extract_claim_values(&raw_id_token, &provider.groups_claim),
    })
  }
}

/// Read a string-or-array claim out of an already-verified JWT payload.
fn extract_claim_values(token: &str, claim: &str) -> Vec<String> {
  let Some(payload) = token.split('.').nth(1) else {
    return Vec::new();
  };
  let Ok(decoded) = URL_SAFE_NO_PAD.decode(payload) else {
    return Vec::new();
  };
  let Ok(value) = serde_json::from_slice::<serde_json::Value>(&decoded) else {
    return Vec::new();
  };

  match value.get(claim) {
    Some(serde_json::Value::Array(items)) => items
      .iter()
      .filter_map(|item| item.as_str().map(str::to_owned))
      .collect(),
    Some(serde_json::Value::String(value)) => value.split_whitespace().map(str::to_owned).collect(),
    _ => Vec::new(),
  }
}

fn select_redirect_uri(
  provider_name: &str,
  provider: &OidcProviderConfig,
  requested: Option<&Url>,
) -> Result<Url, LoginUrlError> {
  if let Some(requested) = requested {
    if provider
      .redirect_uris
      .iter()
      .any(|configured| configured == requested)
    {
      return Ok(requested.clone());
    }

    return Err(LoginUrlError::InvalidRedirectUri {
      provider: provider_name.to_owned(),
      redirect_uri: requested.clone(),
    });
  }

  match provider.redirect_uris.as_slice() {
    [redirect_uri] => Ok(redirect_uri.clone()),
    [] => Err(LoginUrlError::MissingRedirectUri(provider_name.to_owned())),
    _ => Err(LoginUrlError::AmbiguousRedirectUri(
      provider_name.to_owned(),
    )),
  }
}

pub fn generate_state_hash(state: &str) -> String {
  let digest = Sha256::digest(state.as_bytes());
  URL_SAFE_NO_PAD.encode(digest)
}

async fn store_login_transaction(
  redis: &redis::Client,
  transaction: &LoginTransaction,
) -> Result<(), LoginUrlError> {
  let key = format!(
    "{LOGIN_KEY_PREFIX}{}",
    generate_state_hash(&transaction.state)
  );
  let payload = serde_json::to_string(transaction)?;
  let mut connection = redis.get_multiplexed_async_connection().await?;
  connection
    .set_ex::<_, _, ()>(key, payload, LOGIN_TTL_SECONDS)
    .await?;

  Ok(())
}

async fn consume_login_transaction(
  redis: &redis::Client,
  state: &str,
) -> Result<LoginTransaction, LoginUrlError> {
  let key = format!("{LOGIN_KEY_PREFIX}{}", generate_state_hash(state));
  let mut connection = redis.get_multiplexed_async_connection().await?;
  let payload: Option<String> = redis::cmd("GETDEL")
    .arg(key)
    .query_async(&mut connection)
    .await?;
  let payload = payload.ok_or(LoginUrlError::MissingLoginTransaction)?;

  Ok(serde_json::from_str(&payload)?)
}

#[cfg(test)]
mod tests {
  use super::*;
  use redis::AsyncCommands;

  fn test_transaction() -> LoginTransaction {
    LoginTransaction {
      provider: "keycloak".to_string(),
      state: "state-value".to_string(),
      nonce: "nonce-value".to_string(),
      code_verifier: "verifier-value".to_string(),
      redirect_uri: Url::parse("http://localhost:8080/auth/callback").unwrap(),
    }
  }

  #[test]
  fn hashes_state_without_exposing_the_original_value() {
    assert_eq!(
      generate_state_hash("state-value"),
      "prAw7QcteKLKykLonqMhVtJWjsKYigSNm2hM4ecezTs"
    );
    assert_ne!(generate_state_hash("state-value"), "state-value");
  }

  fn token_with_payload(payload: &serde_json::Value) -> String {
    let body = URL_SAFE_NO_PAD.encode(serde_json::to_vec(payload).unwrap());
    format!("header.{body}.signature")
  }

  #[test]
  fn reads_array_and_space_delimited_group_claims() {
    let token = token_with_payload(&serde_json::json!({ "groups": ["a", "b"] }));
    assert_eq!(
      extract_claim_values(&token, "groups"),
      vec!["a".to_string(), "b".to_string()]
    );

    let token = token_with_payload(&serde_json::json!({ "groups": "a b" }));
    assert_eq!(
      extract_claim_values(&token, "groups"),
      vec!["a".to_string(), "b".to_string()]
    );

    let token = token_with_payload(&serde_json::json!({ "other": true }));
    assert!(extract_claim_values(&token, "groups").is_empty());

    assert!(extract_claim_values("not-a-jwt", "groups").is_empty());
  }

  #[tokio::test]
  async fn stores_login_transaction_in_redis_with_ttl() {
    // Only run where Redis is actually configured (CI sets REDIS_URL).
    let Ok(redis_url) = std::env::var("REDIS_URL") else {
      return;
    };
    let redis = redis::Client::open(redis_url).unwrap();
    let transaction = test_transaction();
    let key = format!(
      "{LOGIN_KEY_PREFIX}{}",
      generate_state_hash(&transaction.state)
    );

    store_login_transaction(&redis, &transaction).await.unwrap();

    let mut connection = redis.get_multiplexed_async_connection().await.unwrap();
    let payload: String = connection.get(&key).await.unwrap();
    let stored: LoginTransaction = serde_json::from_str(&payload).unwrap();
    let ttl: i64 = connection.ttl(&key).await.unwrap();

    assert_eq!(stored.provider, transaction.provider);
    assert_eq!(stored.state, transaction.state);
    assert_eq!(stored.nonce, transaction.nonce);
    assert_eq!(stored.code_verifier, transaction.code_verifier);
    assert_eq!(stored.redirect_uri, transaction.redirect_uri);
    assert!((599..=600).contains(&ttl), "unexpected TTL: {ttl}");

    let _: () = connection.del(&key).await.unwrap();
  }
}
