CREATE TABLE users (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  display_name varchar(256) NOT NULL,
  email varchar(320),
  avatar_url varchar(2048),
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  disabled boolean NOT NULL DEFAULT false
);

SELECT diesel_manage_updated_at('users');
