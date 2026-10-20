use std::collections::HashMap;

use serde::{Deserialize, Deserializer};
use thiserror::Error;
use url::Url;

fn default_groups_claim() -> String {
  "groups".to_string()
}

#[derive(Debug, Error)]
pub enum SecretReferenceError {
  #[error("secret reference is empty")]
  EmptyReference,

  #[error("secret environment variable name is empty")]
  EmptyEnvironmentVariableName,

  #[error("secret environment variable {0:?} is not set")]
  EnvironmentVariableNotSet(String),

  #[error("secret file {path:?} could not be read: {source}")]
  DockerSecretRead {
    path: String,

    #[source]
    source: std::io::Error,
  },
}

#[derive(Clone, Eq, PartialEq)]
pub enum SecretReference {
  Embedded(String),
  EnvironmentVariable(String),
  DockerSecret(String),
}

impl std::fmt::Debug for SecretReference {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Embedded(_) => formatter.write_str("SecretReference::Embedded(REDACTED)"),
      Self::EnvironmentVariable(name) => formatter
        .debug_tuple("SecretReference::EnvironmentVariable")
        .field(name)
        .finish(),
      Self::DockerSecret(name) => formatter
        .debug_tuple("SecretReference::DockerSecret")
        .field(name)
        .finish(),
    }
  }
}

impl<'de> Deserialize<'de> for SecretReference {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let reference = String::deserialize(deserializer)?;

    if reference.is_empty() {
      return Err(serde::de::Error::custom(
        SecretReferenceError::EmptyReference,
      ));
    }

    if let Some(name) = reference.strip_prefix("ENV::") {
      if name.is_empty() {
        return Err(serde::de::Error::custom(
          SecretReferenceError::EmptyEnvironmentVariableName,
        ));
      }

      return Ok(Self::EnvironmentVariable(name.to_owned()));
    }

    if let Some(name) = reference.strip_prefix("DOCKER_SECRET::") {
      if name.is_empty() {
        return Err(serde::de::Error::custom(
          SecretReferenceError::EmptyReference,
        ));
      }

      return Ok(Self::DockerSecret(name.to_owned()));
    }

    Ok(Self::Embedded(reference))
  }
}

impl SecretReference {
  pub fn resolve(&self) -> Result<String, SecretReferenceError> {
    match self {
      Self::Embedded(value) => Ok(value.clone()),
      Self::EnvironmentVariable(name) => std::env::var(name)
        .map_err(|_| SecretReferenceError::EnvironmentVariableNotSet(name.clone())),
      Self::DockerSecret(name) => {
        let path = format!("/run/secrets/{name}");
        let value = std::fs::read_to_string(&path).map_err(|source| {
          SecretReferenceError::DockerSecretRead {
            path: path.clone(),
            source,
          }
        })?;

        Ok(value.trim_end_matches(['\r', '\n']).to_owned())
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn deserializes_embedded_secret() {
    let secret: SecretReference = serde_json::from_str("\"embedded-secret\"").unwrap();

    assert_eq!(
      secret,
      SecretReference::Embedded("embedded-secret".to_string())
    );
    assert_eq!(secret.resolve().unwrap(), "embedded-secret");
    assert_eq!(format!("{secret:?}"), "SecretReference::Embedded(REDACTED)");
  }

  #[test]
  fn deserializes_environment_secret_reference() {
    let secret: SecretReference = serde_json::from_str("\"ENV::OIDC_SECRET\"").unwrap();

    assert_eq!(
      secret,
      SecretReference::EnvironmentVariable("OIDC_SECRET".to_string())
    );
    assert_eq!(
      secret.resolve().unwrap_err().to_string(),
      "secret environment variable \"OIDC_SECRET\" is not set"
    );
  }

  #[test]
  fn deserializes_docker_secret_reference() {
    let secret: SecretReference = serde_json::from_str("\"DOCKER_SECRET::oidc-secret\"").unwrap();

    assert_eq!(
      secret,
      SecretReference::DockerSecret("oidc-secret".to_string())
    );
  }

  #[test]
  fn rejects_empty_secret_reference() {
    let error = serde_json::from_str::<SecretReference>("\"\"").unwrap_err();

    assert!(error.to_string().contains("secret reference is empty"));
  }
}

#[derive(Debug, Deserialize)]
pub struct OidcConfig {
  pub default: String,
  pub provider: HashMap<String, OidcProviderConfig>,
}

#[derive(Debug, Deserialize)]
pub struct OidcProviderConfig {
  pub issuer_url: Url,
  pub client_id: String,
  pub client_secret: SecretReference,
  pub redirect_uris: Vec<Url>,
  pub scopes: Vec<String>,

  #[serde(default = "default_groups_claim")]
  pub groups_claim: String,

  #[serde(default)]
  pub mappings: HashMap<String, String>,
}
