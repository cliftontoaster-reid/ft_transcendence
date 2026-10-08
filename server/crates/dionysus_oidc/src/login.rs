use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use openidconnect::{
  ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge, RedirectUrl, Scope,
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

pub struct LoginUrlService {
  config: OidcConfig,
  redis: redis::Client,
}

impl LoginUrlService {
  pub fn new(config: OidcConfig, redis: redis::Client) -> Self {
    Self { config, redis }
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

  #[tokio::test]
  async fn stores_login_transaction_in_redis_with_ttl() {
    let redis = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
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
