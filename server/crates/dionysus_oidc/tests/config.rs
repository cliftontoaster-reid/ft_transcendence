use dionysus_oidc::model::config::OidcConfig;
use url::Url;

#[test]
fn parses_wrapped_provider_configuration() {
  let config: toml::Value = toml::from_str(
    r#"
[oidc]
default = "keycloak"

[oidc.provider.keycloak]
issuer_url = "https://auth.example.com/realms/main"
client_id = "netflix-app"
client_secret = "ENV::OIDC_KEYCLOAK_CLIENT_SECRET"
redirect_uris = ["http://localhost:8000/auth/callback"]
scopes = ["openid", "profile", "email"]

[oidc.provider.keycloak.mappings]
"Admin Group" = "admins"
"#,
  )
  .unwrap();

  let config: OidcConfig = config
    .get("oidc")
    .cloned()
    .map(toml::Value::try_into)
    .unwrap()
    .unwrap();
  let provider = config.provider.get("keycloak").unwrap();

  assert_eq!(config.default, "keycloak");
  assert_eq!(
    provider.redirect_uris,
    vec![Url::parse("http://localhost:8000/auth/callback").unwrap()]
  );
  assert_eq!(
    provider.mappings.get("Admin Group"),
    Some(&"admins".to_string())
  );
}

#[test]
fn parses_discord_oidc_configuration() {
  let config: toml::Value =
    toml::from_str(include_str!("../../../config/oidc.example.toml")).unwrap();

  let config: OidcConfig = config
    .get("oidc")
    .cloned()
    .map(toml::Value::try_into)
    .unwrap()
    .unwrap();
  let provider = config.provider.get("discord").unwrap();

  assert_eq!(config.default, "discord");
  assert_eq!(
    provider.issuer_url,
    Url::parse("https://discord.com").unwrap()
  );
  assert_eq!(provider.client_id, "REPLACE_WITH_DISCORD_CLIENT_ID");
  assert_eq!(provider.scopes, vec!["openid", "identify"]);
  assert_eq!(
    provider.client_secret,
    dionysus_oidc::model::config::SecretReference::EnvironmentVariable(
      "DISCORD_CLIENT_SECRET".to_string()
    )
  );
}
