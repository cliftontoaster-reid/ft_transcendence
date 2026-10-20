use crate::{
  schema::{
    media, media_images, media_track, media_translations, my_list, oidc_identities, permissions,
    role_permissions, roles, sessions, user_roles, users, viewer_profiles, watch_history,
    watch_progress,
  },
  types::{media_image_type::MediaImageType, media_track_type::MediaTrackType, tsvector::TsVector},
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use pgvector::Vector as PgVector;
use uuid::Uuid;

// --- OIDC identities ---
#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(User))]
#[diesel(table_name = oidc_identities)]
pub struct OidcIdentity {
  pub id: Uuid,
  pub user_id: Uuid,
  pub issuer: String,
  pub subject: String,
  pub email: Option<String>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = oidc_identities)]
pub struct NewOidcIdentity<'a> {
  pub user_id: Uuid,
  pub issuer: &'a str,
  pub subject: &'a str,
  pub email: Option<&'a str>,
}

// --- Roles and permissions ---
#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = roles)]
pub struct Role {
  pub id: Uuid,
  pub name: String,
  pub description: Option<String>,
  pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = roles)]
pub struct NewRole<'a> {
  pub name: &'a str,
  pub description: Option<&'a str>,
}

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = permissions)]
pub struct Permission {
  pub id: Uuid,
  pub name: String,
  pub description: Option<String>,
  pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = permissions)]
pub struct NewPermission<'a> {
  pub name: &'a str,
  pub description: Option<&'a str>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(primary_key(user_id, role_id))]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Role))]
#[diesel(table_name = user_roles)]
pub struct UserRole {
  pub user_id: Uuid,
  pub role_id: Uuid,
  pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = user_roles)]
pub struct NewUserRole {
  pub user_id: Uuid,
  pub role_id: Uuid,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(primary_key(role_id, permission_id))]
#[diesel(belongs_to(Role))]
#[diesel(belongs_to(Permission))]
#[diesel(table_name = role_permissions)]
pub struct RolePermission {
  pub role_id: Uuid,
  pub permission_id: Uuid,
  pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = role_permissions)]
pub struct NewRolePermission {
  pub role_id: Uuid,
  pub permission_id: Uuid,
}

// --- Users ---
#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = users)]
pub struct User {
  pub id: Uuid,
  pub display_name: String,
  pub email: Option<String>,
  pub avatar_url: Option<String>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub disabled: bool,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
  pub display_name: &'a str,
  pub email: Option<&'a str>,
  pub avatar_url: Option<&'a str>,
}

// --- Viewer profiles ---
#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(User))]
#[diesel(table_name = viewer_profiles)]
pub struct ViewerProfile {
  pub id: Uuid,
  pub user_id: Uuid,
  pub display_name: String,
  pub avatar_url: Option<String>,
  pub is_default: bool,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = viewer_profiles)]
pub struct NewViewerProfile<'a> {
  pub user_id: Uuid,
  pub display_name: &'a str,
  pub avatar_url: Option<&'a str>,
  pub is_default: bool,
}

// --- Sessions ---
#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(ViewerProfile, foreign_key = active_profile_id))]
#[diesel(table_name = sessions)]
pub struct Session {
  pub id: Uuid,
  pub user_id: Uuid,
  pub active_profile_id: Option<Uuid>,
  pub jti: Uuid,
  pub issued_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
  pub last_seen_at: DateTime<Utc>,
  pub revoked_at: Option<DateTime<Utc>>,
  pub user_agent: Option<String>,
  pub ip_address: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = sessions)]
pub struct NewSession<'a> {
  pub user_id: Uuid,
  pub active_profile_id: Option<Uuid>,
  pub jti: Uuid,
  pub expires_at: DateTime<Utc>,
  pub user_agent: Option<&'a str>,
  pub ip_address: Option<&'a str>,
}

// --- Watch state ---
#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(primary_key(profile_id, media_id))]
#[diesel(belongs_to(ViewerProfile, foreign_key = profile_id))]
#[diesel(belongs_to(Media))]
#[diesel(table_name = watch_progress)]
pub struct WatchProgress {
  pub profile_id: Uuid,
  pub media_id: Uuid,
  pub position_ms: i64,
  pub duration_ms: Option<i64>,
  pub completed: bool,
  pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = watch_progress)]
pub struct NewWatchProgress {
  pub profile_id: Uuid,
  pub media_id: Uuid,
  pub position_ms: i64,
  pub duration_ms: Option<i64>,
  pub completed: bool,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(ViewerProfile, foreign_key = profile_id))]
#[diesel(belongs_to(Media))]
#[diesel(table_name = watch_history)]
pub struct WatchHistory {
  pub id: Uuid,
  pub profile_id: Uuid,
  pub media_id: Uuid,
  pub started_at: DateTime<Utc>,
  pub watched_at: DateTime<Utc>,
  pub position_ms: i64,
  pub duration_ms: Option<i64>,
}

#[derive(Insertable)]
#[diesel(table_name = watch_history)]
pub struct NewWatchHistory {
  pub profile_id: Uuid,
  pub media_id: Uuid,
  pub position_ms: i64,
  pub duration_ms: Option<i64>,
}

// --- My List ---
#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(primary_key(profile_id, media_id))]
#[diesel(belongs_to(ViewerProfile, foreign_key = profile_id))]
#[diesel(belongs_to(Media))]
#[diesel(table_name = my_list)]
pub struct MyListEntry {
  pub profile_id: Uuid,
  pub media_id: Uuid,
  pub added_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = my_list)]
pub struct NewMyListEntry {
  pub profile_id: Uuid,
  pub media_id: Uuid,
}

// --- Media ---
#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = media)]
pub struct Media {
  pub id: Uuid,
  pub manifest_path: String,
}

#[derive(Insertable)]
#[diesel(table_name = media)]
pub struct NewMedia<'a> {
  pub manifest_path: &'a str,
}

// --- Media Translations ---
#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(Media))]
#[diesel(table_name = media_translations)]
pub struct MediaTranslation {
  pub id: Uuid,
  pub media_id: Uuid,
  pub locale: String,
  pub title: String,
  pub synopsis: String,
  pub summary: String,
  pub fts_content: Option<TsVector>,
  pub embedding: Option<PgVector>,
}

#[derive(Insertable)]
#[diesel(table_name = media_translations)]
pub struct NewMediaTranslation<'a> {
  pub media_id: Uuid,
  pub locale: &'a str,
  pub title: &'a str,
  pub synopsis: &'a str,
  pub summary: &'a str,
  pub embedding: Option<PgVector>,
}

// --- Media Track ---
#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(Media))]
#[diesel(table_name = media_track)]
pub struct MediaTrack {
  pub id: Uuid,
  pub media_id: Uuid,
  pub track_type: MediaTrackType,
  pub locale: String,
  pub storage_path: String,
}

#[derive(Insertable)]
#[diesel(table_name = media_track)]
pub struct NewMediaTrack<'a> {
  pub media_id: Uuid,
  pub track_type: MediaTrackType,
  pub locale: &'a str,
  pub storage_path: &'a str,
}

// --- Media Images ---
#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(Media))]
#[diesel(table_name = media_images)]
pub struct MediaImage {
  pub id: Uuid,
  pub media_id: Uuid,
  #[diesel(column_name = type_)]
  pub image_type: MediaImageType,
  pub locale: String,
  pub storage_path: String,
}

#[derive(Insertable)]
#[diesel(table_name = media_images)]
pub struct NewMediaImage<'a> {
  pub media_id: Uuid,
  #[diesel(column_name = type_)]
  pub image_type: MediaImageType,
  pub locale: &'a str,
  pub storage_path: &'a str,
}
