-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS vector;
CREATE TABLE IF NOT EXISTS media (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  manifest_path varchar(255) NOT NULL
);
CREATE TABLE IF NOT EXISTS media_translations (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  locale varchar(35) NOT NULL,
  title varchar(256) NOT NULL,
  synopsis varchar(500) NOT NULL,
  summary varchar(4000) NOT NULL,
  fts_content tsvector GENERATED ALWAYS AS (
    to_tsvector(
      'simple'::regconfig,
      coalesce(title, '') || ' ' || coalesce(synopsis, '') || ' ' || coalesce(summary, '')
    )
  ) STORED,
  embedding vector(768)
);
CREATE TYPE media_image_type AS ENUM (
  'poster',
  'backdrop',
  'logo',
  'banner',
  'still',
  'thumbnail'
);
CREATE TABLE IF NOT EXISTS media_images (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  type media_image_type NOT NULL,
  locale varchar(35) NOT NULL,
  storage_path varchar(255) NOT NULL
);
CREATE INDEX IF NOT EXISTS media_images_media_id_idx ON media_images (media_id);