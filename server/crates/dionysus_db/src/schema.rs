// @generated automatically by Diesel CLI.

pub mod sql_types {
  #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
  #[diesel(postgres_type(name = "media_image_type"))]
  pub struct MediaImageType;

  #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
  #[diesel(postgres_type(name = "media_track_type"))]
  pub struct MediaTrackType;

  #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
  #[diesel(postgres_type(name = "tsvector", schema = "pg_catalog"))]
  pub struct Tsvector;
}

diesel::table! {
  media (id) {
    id -> Uuid,
    #[max_length = 255]
    manifest_path -> Varchar,
  }
}

diesel::table! {
  use diesel::sql_types::*;
  use super::sql_types::MediaImageType;

  media_images (id) {
    id -> Uuid,
    media_id -> Uuid,
    #[sql_name = "type"]
    type_ -> MediaImageType,
    #[max_length = 35]
    locale -> Varchar,
    #[max_length = 255]
    storage_path -> Varchar,
  }
}

diesel::table! {
  use diesel::sql_types::*;
  use super::sql_types::MediaTrackType;

  media_track (id) {
    id -> Uuid,
    media_id -> Uuid,
    track_type -> MediaTrackType,
    #[max_length = 35]
    locale -> Varchar,
    #[max_length = 255]
    storage_path -> Varchar,
  }
}

diesel::table! {
  use diesel::sql_types::*;
  use super::sql_types::Tsvector;
  use pgvector::sql_types::Vector;

  media_translations (id) {
    id -> Uuid,
    media_id -> Uuid,
    #[max_length = 35]
    locale -> Varchar,
    #[max_length = 256]
    title -> Varchar,
    #[max_length = 500]
    synopsis -> Varchar,
    #[max_length = 4000]
    summary -> Varchar,
    fts_content -> Nullable<Tsvector>,
    embedding -> Nullable<Vector>,
  }
}

diesel::table! {
  oidc_identities (id) {
    id -> Uuid,
    user_id -> Uuid,
    #[max_length = 2048]
    issuer -> Varchar,
    #[max_length = 255]
    subject -> Varchar,
    #[max_length = 320]
    email -> Nullable<Varchar>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
  }
}

diesel::table! {
  permissions (id) {
    id -> Uuid,
    #[max_length = 128]
    name -> Varchar,
    #[max_length = 256]
    description -> Nullable<Varchar>,
    created_at -> Timestamptz,
  }
}

diesel::table! {
  role_permissions (role_id, permission_id) {
    role_id -> Uuid,
    permission_id -> Uuid,
    created_at -> Timestamptz,
  }
}

diesel::table! {
  roles (id) {
    id -> Uuid,
    #[max_length = 64]
    name -> Varchar,
    #[max_length = 256]
    description -> Nullable<Varchar>,
    created_at -> Timestamptz,
  }
}

diesel::table! {
  sessions (id) {
    id -> Uuid,
    user_id -> Uuid,
    active_profile_id -> Nullable<Uuid>,
    jti -> Uuid,
    issued_at -> Timestamptz,
    expires_at -> Timestamptz,
    last_seen_at -> Timestamptz,
    revoked_at -> Nullable<Timestamptz>,
    #[max_length = 1024]
    user_agent -> Nullable<Varchar>,
    ip_address -> Nullable<Inet>,
  }
}

diesel::table! {
  user_roles (user_id, role_id) {
    user_id -> Uuid,
    role_id -> Uuid,
    created_at -> Timestamptz,
  }
}

diesel::table! {
  users (id) {
    id -> Uuid,
    #[max_length = 256]
    display_name -> Varchar,
    #[max_length = 320]
    email -> Nullable<Varchar>,
    #[max_length = 2048]
    avatar_url -> Nullable<Varchar>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    disabled -> Bool,
  }
}

diesel::table! {
  viewer_profiles (id) {
    id -> Uuid,
    user_id -> Uuid,
    #[max_length = 256]
    display_name -> Varchar,
    #[max_length = 2048]
    avatar_url -> Nullable<Varchar>,
    is_default -> Bool,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
  }
}

diesel::table! {
  my_list (profile_id, media_id) {
    profile_id -> Uuid,
    media_id -> Uuid,
    added_at -> Timestamptz,
  }
}

diesel::table! {
  watch_history (id) {
    id -> Uuid,
    profile_id -> Uuid,
    media_id -> Uuid,
    started_at -> Timestamptz,
    watched_at -> Timestamptz,
    position_ms -> Int8,
    duration_ms -> Nullable<Int8>,
  }
}

diesel::table! {
  watch_progress (profile_id, media_id) {
    profile_id -> Uuid,
    media_id -> Uuid,
    position_ms -> Int8,
    duration_ms -> Nullable<Int8>,
    completed -> Bool,
    updated_at -> Timestamptz,
  }
}

diesel::joinable!(my_list -> media (media_id));
diesel::joinable!(my_list -> viewer_profiles (profile_id));
diesel::joinable!(media_images -> media (media_id));
diesel::joinable!(media_track -> media (media_id));
diesel::joinable!(media_translations -> media (media_id));
diesel::joinable!(oidc_identities -> users (user_id));
diesel::joinable!(role_permissions -> permissions (permission_id));
diesel::joinable!(role_permissions -> roles (role_id));
diesel::joinable!(sessions -> users (user_id));
diesel::joinable!(sessions -> viewer_profiles (active_profile_id));
diesel::joinable!(user_roles -> roles (role_id));
diesel::joinable!(user_roles -> users (user_id));
diesel::joinable!(viewer_profiles -> users (user_id));
diesel::joinable!(watch_history -> media (media_id));
diesel::joinable!(watch_history -> viewer_profiles (profile_id));
diesel::joinable!(watch_progress -> media (media_id));
diesel::joinable!(watch_progress -> viewer_profiles (profile_id));

diesel::allow_tables_to_appear_in_same_query!(
  media,
  media_images,
  media_track,
  media_translations,
  my_list,
  oidc_identities,
  permissions,
  role_permissions,
  roles,
  sessions,
  user_roles,
  users,
  viewer_profiles,
  watch_history,
  watch_progress,
);
