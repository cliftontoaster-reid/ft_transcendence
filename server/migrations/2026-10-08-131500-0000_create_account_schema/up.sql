CREATE TABLE oidc_identities (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  issuer varchar(2048) NOT NULL,
  subject varchar(255) NOT NULL,
  email varchar(320),
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (issuer, subject)
);
CREATE INDEX oidc_identities_user_id_idx ON oidc_identities (user_id);
SELECT diesel_manage_updated_at('oidc_identities');
CREATE TABLE roles (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name varchar(64) NOT NULL UNIQUE,
  description varchar(256),
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE permissions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name varchar(128) NOT NULL UNIQUE,
  description varchar(256),
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE user_roles (
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role_id uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (user_id, role_id)
);
CREATE TABLE role_permissions (
  role_id uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
  permission_id uuid NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (role_id, permission_id)
);
CREATE TABLE viewer_profiles (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  display_name varchar(256) NOT NULL,
  avatar_url varchar(2048),
  is_default boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (user_id, display_name)
);
CREATE UNIQUE INDEX viewer_profiles_one_default_idx ON viewer_profiles (user_id)
WHERE is_default;
SELECT diesel_manage_updated_at('viewer_profiles');
CREATE TABLE sessions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  active_profile_id uuid REFERENCES viewer_profiles(id) ON DELETE
  SET NULL,
    jti uuid NOT NULL UNIQUE,
    issued_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at timestamptz NOT NULL,
    last_seen_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at timestamptz,
    user_agent varchar(1024),
    ip_address inet
);
CREATE INDEX sessions_user_id_idx ON sessions (user_id);
CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);
CREATE TABLE watch_progress (
  profile_id uuid NOT NULL REFERENCES viewer_profiles(id) ON DELETE CASCADE,
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  position_ms bigint NOT NULL DEFAULT 0 CHECK (position_ms >= 0),
  duration_ms bigint CHECK (
    duration_ms IS NULL
    OR duration_ms > 0
  ),
  completed boolean NOT NULL DEFAULT false,
  updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (profile_id, media_id)
);
CREATE TABLE watch_history (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  profile_id uuid NOT NULL REFERENCES viewer_profiles(id) ON DELETE CASCADE,
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  started_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  watched_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  position_ms bigint NOT NULL DEFAULT 0 CHECK (position_ms >= 0),
  duration_ms bigint CHECK (
    duration_ms IS NULL
    OR duration_ms > 0
  )
);
CREATE INDEX watch_history_profile_watched_at_idx ON watch_history (profile_id, watched_at DESC);
CREATE TABLE my_list (
  profile_id uuid NOT NULL REFERENCES viewer_profiles(id) ON DELETE CASCADE,
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  added_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (profile_id, media_id)
);