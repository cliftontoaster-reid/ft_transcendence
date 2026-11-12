use std::collections::HashMap;

use chrono::Duration;
use diesel::{Connection, PgConnection, prelude::*};
use dionysus_db::schema::{oidc_identities, roles, user_roles, users, viewer_profiles};
use dionysus_oidc::{
  account::provision_account,
  login::AuthenticatedUser,
  model::config::{OidcProviderConfig, SecretReference},
  session::{SessionConfig, SessionError, SessionService},
};
use url::Url;
use uuid::Uuid;

const ISSUER: &str = "https://auth.example.test/realms/main";
const SESSION_SECRET: &[u8] = b"a-session-secret-that-is-long-enough!";

struct Fixture {
  connection: PgConnection,
  role_name: String,
}

fn fixture() -> Option<Fixture> {
  let database_url = std::env::var("DATABASE_URL").ok()?;
  let connection = PgConnection::establish(&database_url).ok()?;

  Some(Fixture {
    connection,
    role_name: format!("test-role-{}", Uuid::new_v4()),
  })
}

fn provider(role_name: &str) -> OidcProviderConfig {
  let mut mappings = HashMap::new();
  mappings.insert("Test Admins".to_string(), role_name.to_string());

  OidcProviderConfig {
    issuer_url: Url::parse(ISSUER).unwrap(),
    client_id: "test-client".to_string(),
    client_secret: SecretReference::Embedded("test-secret".to_string()),
    redirect_uris: vec![Url::parse("http://localhost:8000/auth/callback").unwrap()],
    scopes: vec!["openid".to_string()],
    groups_claim: "groups".to_string(),
    mappings,
  }
}

fn identity(tag: &str) -> AuthenticatedUser {
  let unique = Uuid::new_v4();

  AuthenticatedUser {
    provider: "test".to_string(),
    issuer: ISSUER.to_string(),
    subject: format!("subject-{tag}-{unique}"),
    email: Some(format!("{tag}-{unique}@example.test")),
    email_verified: Some(true),
    preferred_username: Some(format!("{tag}-{unique}")),
    groups: vec!["Test Admins".to_string()],
  }
}

fn cleanup(connection: &mut PgConnection, user_ids: &[Uuid], role_names: &[&str]) {
  let _ = diesel::delete(users::table.filter(users::id.eq_any(user_ids))).execute(connection);
  for role_name in role_names {
    let _ = diesel::delete(roles::table.filter(roles::name.eq(*role_name))).execute(connection);
  }
}

#[test]
fn provisions_then_reuses_an_account() {
  let Some(mut fixture) = fixture() else {
    return;
  };
  let role_name = fixture.role_name.clone();
  let provider = provider(&role_name);
  let identity = identity("provision");

  let first = provision_account(&mut fixture.connection, &identity, &provider).unwrap();
  assert!(first.created);
  assert_eq!(first.roles, vec![role_name.clone()]);
  assert_eq!(
    first.display_name,
    identity.preferred_username.clone().unwrap()
  );
  assert_eq!(first.email, identity.email);

  let identity_count: i64 = oidc_identities::table
    .filter(oidc_identities::user_id.eq(first.user_id))
    .count()
    .get_result(&mut fixture.connection)
    .unwrap();
  assert_eq!(identity_count, 1);

  let default_profiles: i64 = viewer_profiles::table
    .filter(viewer_profiles::user_id.eq(first.user_id))
    .filter(viewer_profiles::is_default.eq(true))
    .count()
    .get_result(&mut fixture.connection)
    .unwrap();
  assert_eq!(default_profiles, 1);

  let assigned_roles: Vec<String> = user_roles::table
    .inner_join(roles::table)
    .filter(user_roles::user_id.eq(first.user_id))
    .select(roles::name)
    .load(&mut fixture.connection)
    .unwrap();
  assert_eq!(assigned_roles, vec![role_name.clone()]);

  let second = provision_account(&mut fixture.connection, &identity, &provider).unwrap();
  assert!(!second.created);
  assert_eq!(second.user_id, first.user_id);

  let identity_count: i64 = oidc_identities::table
    .filter(oidc_identities::user_id.eq(first.user_id))
    .count()
    .get_result(&mut fixture.connection)
    .unwrap();
  assert_eq!(identity_count, 1);

  cleanup(&mut fixture.connection, &[first.user_id], &[&role_name]);
}

#[test]
fn links_a_second_identity_through_a_verified_email() {
  let Some(mut fixture) = fixture() else {
    return;
  };
  let role_name = fixture.role_name.clone();
  let provider = provider(&role_name);
  let first_identity = identity("link");

  let first = provision_account(&mut fixture.connection, &first_identity, &provider).unwrap();

  let mut second_identity = identity("link");
  second_identity.email = first_identity.email.clone();
  let second = provision_account(&mut fixture.connection, &second_identity, &provider).unwrap();

  assert!(!second.created);
  assert_eq!(second.user_id, first.user_id);

  cleanup(&mut fixture.connection, &[first.user_id], &[&role_name]);
}

#[test]
fn never_links_an_unverified_email() {
  let Some(mut fixture) = fixture() else {
    return;
  };
  let role_name = fixture.role_name.clone();
  let provider = provider(&role_name);
  let first_identity = identity("unverified");

  let first = provision_account(&mut fixture.connection, &first_identity, &provider).unwrap();

  let mut second_identity = identity("unverified");
  second_identity.email = first_identity.email.clone();
  second_identity.email_verified = Some(false);
  let second = provision_account(&mut fixture.connection, &second_identity, &provider).unwrap();

  assert!(second.created);
  assert_ne!(second.user_id, first.user_id);
  assert_eq!(second.email, None);

  cleanup(
    &mut fixture.connection,
    &[first.user_id, second.user_id],
    &[&role_name],
  );
}

#[test]
fn issues_verifies_and_revokes_sessions() {
  let Some(mut fixture) = fixture() else {
    return;
  };
  let role_name = fixture.role_name.clone();
  let provider = provider(&role_name);
  let account =
    provision_account(&mut fixture.connection, &identity("session"), &provider).unwrap();

  let sessions = SessionService::new(SessionConfig {
    secret: SESSION_SECRET.to_vec(),
    ttl: Duration::hours(1),
    secure_cookie: false,
  })
  .unwrap();

  let issued = sessions
    .issue(
      &mut fixture.connection,
      account.user_id,
      Some(account.profile_id),
      Some("test-agent"),
    )
    .unwrap();

  let verified = sessions
    .verify(&mut fixture.connection, &issued.token)
    .unwrap();
  assert_eq!(verified.user_id, account.user_id);
  assert_eq!(verified.session_id, issued.session_id);
  assert_eq!(verified.active_profile_id, Some(account.profile_id));

  sessions
    .revoke(&mut fixture.connection, issued.jti)
    .unwrap();
  assert!(matches!(
    sessions.verify(&mut fixture.connection, &issued.token),
    Err(SessionError::Revoked)
  ));

  let other = SessionService::new(SessionConfig {
    secret: b"a-different-secret-that-is-long-enough".to_vec(),
    ttl: Duration::hours(1),
    secure_cookie: false,
  })
  .unwrap();
  assert!(matches!(
    other.verify(&mut fixture.connection, &issued.token),
    Err(SessionError::Token(_))
  ));

  diesel::update(users::table.find(account.user_id))
    .set(users::disabled.eq(true))
    .execute(&mut fixture.connection)
    .unwrap();

  let issued = sessions
    .issue(&mut fixture.connection, account.user_id, None, None)
    .unwrap();
  assert!(matches!(
    sessions.verify(&mut fixture.connection, &issued.token),
    Err(SessionError::Disabled)
  ));

  cleanup(&mut fixture.connection, &[account.user_id], &[&role_name]);
}
