use chrono::Utc;
use diesel::{
  prelude::*,
  result::{DatabaseErrorKind, Error as DieselError},
};
use dionysus_db::schema::{oidc_identities, roles, user_roles, users, viewer_profiles};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::{login::AuthenticatedUser, model::config::OidcProviderConfig};

const MAX_DISPLAY_NAME: usize = 256;
const MAX_USERNAME: usize = 64;
const MAX_ROLE_NAME: usize = 64;

#[derive(Debug, Error)]
pub enum AccountError {
  #[error("database error: {0}")]
  Database(#[from] DieselError),

  #[error("user account {0} is disabled")]
  Disabled(Uuid),

  #[error("identity was linked to a user that no longer exists")]
  MissingUser,

  #[error("display name is empty")]
  EmptyDisplayName,

  #[error("mapped role name {0:?} is invalid")]
  InvalidRoleName(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ProvisionedAccount {
  pub user_id: Uuid,
  pub display_name: String,
  pub email: Option<String>,
  pub username: Option<String>,
  pub profile_id: Uuid,
  pub roles: Vec<String>,
  /// `true` when this login created the user row.
  pub created: bool,
}

/// Find, link, or create the local account behind an OIDC identity, then make
/// sure it has a default viewer profile and its mapped roles.
pub fn provision_account(
  conn: &mut PgConnection,
  identity: &AuthenticatedUser,
  provider: &OidcProviderConfig,
) -> Result<ProvisionedAccount, AccountError> {
  let existing_user_id: Option<Uuid> = oidc_identities::table
    .filter(oidc_identities::issuer.eq(&identity.issuer))
    .filter(oidc_identities::subject.eq(&identity.subject))
    .select(oidc_identities::user_id)
    .first(conn)
    .optional()?;

  let (user_id, created) = match existing_user_id {
    Some(user_id) => {
      diesel::update(
        oidc_identities::table
          .filter(oidc_identities::issuer.eq(&identity.issuer))
          .filter(oidc_identities::subject.eq(&identity.subject)),
      )
      .set((
        oidc_identities::email.eq(identity.email.as_deref()),
        oidc_identities::updated_at.eq(Utc::now()),
      ))
      .execute(conn)?;
      (user_id, false)
    }
    None => create_or_link_user(conn, identity)?,
  };

  let (display_name, mut email, username, disabled): (
    String,
    Option<String>,
    Option<String>,
    bool,
  ) = users::table
    .find(user_id)
    .select((
      users::display_name,
      users::email,
      users::username,
      users::disabled,
    ))
    .first(conn)
    .optional()?
    .ok_or(AccountError::MissingUser)?;

  if disabled {
    return Err(AccountError::Disabled(user_id));
  }

  // Backfill a missing email from a trusted identity claim.
  if email.is_none()
    && let Some(trusted_email) = identity.trustworthy_email()
  {
    diesel::update(users::table.find(user_id))
      .set(users::email.eq(trusted_email))
      .execute(conn)?;
    email = Some(trusted_email.to_string());
  }

  let profile_id = ensure_default_profile(conn, user_id, &display_name)?;
  let assigned_roles = sync_roles(conn, user_id, provider, &identity.groups)?;

  Ok(ProvisionedAccount {
    user_id,
    display_name,
    email,
    username,
    profile_id,
    roles: assigned_roles,
    created,
  })
}

fn create_or_link_user(
  conn: &mut PgConnection,
  identity: &AuthenticatedUser,
) -> Result<(Uuid, bool), AccountError> {
  // Only link to an existing account through a verified email. An unverified
  // or explicitly untrusted email must never take over someone else's account.
  if let Some(email) = identity.trustworthy_email() {
    let candidates: Vec<Uuid> = users::table
      .filter(users::email.eq(email))
      .select(users::id)
      .limit(2)
      .load(conn)?;

    if let [user_id] = candidates.as_slice() {
      let user_id = *user_id;
      let linked_user_id = insert_identity(conn, user_id, identity)?;
      return Ok((linked_user_id, false));
    }
  }

  let user_id = insert_user(conn, identity)?;
  let linked_user_id = insert_identity(conn, user_id, identity)?;

  // A concurrent login may have claimed the identity first. Drop the user we
  // just created and reuse the winner's account instead.
  if linked_user_id != user_id {
    diesel::delete(users::table.find(user_id)).execute(conn)?;
    return Ok((linked_user_id, false));
  }

  Ok((user_id, true))
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct NewUserRow<'a> {
  display_name: &'a str,
  email: Option<&'a str>,
  username: Option<&'a str>,
}

fn insert_user(
  conn: &mut PgConnection,
  identity: &AuthenticatedUser,
) -> Result<Uuid, AccountError> {
  let display_name = truncate(identity.display_name_hint(), MAX_DISPLAY_NAME);
  if display_name.trim().is_empty() {
    return Err(AccountError::EmptyDisplayName);
  }

  let email = identity.trustworthy_email();
  let username = identity
    .preferred_username
    .as_deref()
    .and_then(sanitize_username);

  let insert = |conn: &mut PgConnection, username: Option<&str>| {
    diesel::insert_into(users::table)
      .values(NewUserRow {
        display_name: &display_name,
        email,
        username,
      })
      .returning(users::id)
      .get_result::<Uuid>(conn)
  };

  match insert(conn, username.as_deref()) {
    Ok(user_id) => Ok(user_id),
    // The preferred username is cosmetic; retry without it when it is taken.
    Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _))
      if username.is_some() =>
    {
      insert(conn, None).map_err(Into::into)
    }
    Err(error) => Err(error.into()),
  }
}

#[derive(Insertable)]
#[diesel(table_name = oidc_identities)]
struct NewIdentityRow<'a> {
  user_id: Uuid,
  issuer: &'a str,
  subject: &'a str,
  email: Option<&'a str>,
}

fn insert_identity(
  conn: &mut PgConnection,
  user_id: Uuid,
  identity: &AuthenticatedUser,
) -> Result<Uuid, AccountError> {
  diesel::insert_into(oidc_identities::table)
    .values(NewIdentityRow {
      user_id,
      issuer: &identity.issuer,
      subject: &identity.subject,
      email: identity.email.as_deref(),
    })
    .on_conflict((oidc_identities::issuer, oidc_identities::subject))
    .do_nothing()
    .execute(conn)?;

  oidc_identities::table
    .filter(oidc_identities::issuer.eq(&identity.issuer))
    .filter(oidc_identities::subject.eq(&identity.subject))
    .select(oidc_identities::user_id)
    .first(conn)
    .map_err(Into::into)
}

#[derive(Insertable)]
#[diesel(table_name = viewer_profiles)]
struct NewProfileRow<'a> {
  user_id: Uuid,
  display_name: &'a str,
  is_default: bool,
}

fn ensure_default_profile(
  conn: &mut PgConnection,
  user_id: Uuid,
  display_name: &str,
) -> Result<Uuid, AccountError> {
  let existing: Option<Uuid> = viewer_profiles::table
    .filter(viewer_profiles::user_id.eq(user_id))
    .order((
      viewer_profiles::is_default.desc(),
      viewer_profiles::created_at.asc(),
    ))
    .select(viewer_profiles::id)
    .first(conn)
    .optional()?;

  if let Some(profile_id) = existing {
    return Ok(profile_id);
  }

  let profile_name = truncate(display_name.to_owned(), MAX_DISPLAY_NAME);

  diesel::insert_into(viewer_profiles::table)
    .values(NewProfileRow {
      user_id,
      display_name: &profile_name,
      is_default: true,
    })
    .returning(viewer_profiles::id)
    .get_result(conn)
    .map_err(Into::into)
}

fn sync_roles(
  conn: &mut PgConnection,
  user_id: Uuid,
  provider: &OidcProviderConfig,
  groups: &[String],
) -> Result<Vec<String>, AccountError> {
  let mut assigned = Vec::new();

  for role_name in provider.mapped_roles(groups) {
    if role_name.is_empty() || role_name.chars().count() > MAX_ROLE_NAME {
      return Err(AccountError::InvalidRoleName(role_name));
    }

    let role_id: Uuid = diesel::insert_into(roles::table)
      .values((
        roles::name.eq(&role_name),
        roles::description.eq(Option::<String>::None),
      ))
      .on_conflict(roles::name)
      // Re-asserting the same name is a no-op that still returns the id.
      .do_update()
      .set(roles::name.eq(&role_name))
      .returning(roles::id)
      .get_result(conn)?;

    diesel::insert_into(user_roles::table)
      .values((
        user_roles::user_id.eq(user_id),
        user_roles::role_id.eq(role_id),
      ))
      .on_conflict_do_nothing()
      .execute(conn)?;

    assigned.push(role_name);
  }

  Ok(assigned)
}

fn sanitize_username(raw: &str) -> Option<String> {
  let cleaned: String = raw
    .trim()
    .chars()
    .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    .take(MAX_USERNAME)
    .collect();

  (!cleaned.is_empty()).then_some(cleaned)
}

fn truncate(value: String, max_chars: usize) -> String {
  if value.chars().count() <= max_chars {
    value
  } else {
    value.chars().take(max_chars).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;

  fn identity() -> AuthenticatedUser {
    AuthenticatedUser {
      provider: "keycloak".to_string(),
      issuer: "https://auth.example.com/realms/main".to_string(),
      subject: "subject".to_string(),
      email: Some("user@example.com".to_string()),
      email_verified: Some(true),
      preferred_username: Some("user".to_string()),
      groups: vec!["Admin Group".to_string()],
    }
  }

  #[test]
  fn trusts_verified_and_unstated_emails_only() {
    let mut identity = identity();
    assert_eq!(identity.trustworthy_email(), Some("user@example.com"));

    identity.email_verified = None;
    assert_eq!(identity.trustworthy_email(), Some("user@example.com"));

    identity.email_verified = Some(false);
    assert_eq!(identity.trustworthy_email(), None);
  }

  #[test]
  fn builds_display_name_from_username_email_then_subject() {
    let mut identity = identity();
    assert_eq!(identity.display_name_hint(), "user");

    identity.preferred_username = Some("   ".to_string());
    assert_eq!(identity.display_name_hint(), "user");

    identity.email = None;
    assert_eq!(identity.display_name_hint(), "keycloak:subject");
  }

  #[test]
  fn maps_groups_to_deduplicated_sorted_roles() {
    let provider = provider_config();

    assert_eq!(
      provider.mapped_roles(["Admin Group", "Viewer", "Unknown"]),
      vec!["admin".to_string(), "viewer".to_string()]
    );
  }

  #[test]
  fn sanitizes_usernames_and_bounds_their_length() {
    assert_eq!(
      sanitize_username("  jane.doe-1 "),
      Some("jane.doe-1".to_string())
    );
    assert_eq!(sanitize_username("💥"), None);

    let long = "a".repeat(100);
    assert_eq!(sanitize_username(&long).unwrap().len(), MAX_USERNAME);
  }

  fn provider_config() -> OidcProviderConfig {
    let mut mappings = HashMap::new();
    mappings.insert("Admin Group".to_string(), "admin".to_string());
    mappings.insert("Viewer".to_string(), "viewer".to_string());
    mappings.insert("Duplicate".to_string(), "admin".to_string());

    OidcProviderConfig {
      issuer_url: url::Url::parse("https://auth.example.com/realms/main").unwrap(),
      client_id: "client".to_string(),
      client_secret: crate::model::config::SecretReference::Embedded("secret".to_string()),
      redirect_uris: vec![url::Url::parse("http://localhost:8000/auth/callback").unwrap()],
      scopes: vec!["openid".to_string()],
      groups_claim: "groups".to_string(),
      mappings,
    }
  }
}
