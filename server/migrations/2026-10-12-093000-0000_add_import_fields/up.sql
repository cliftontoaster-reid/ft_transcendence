-- Media import pipeline: provenance/dedup on media, richer track metadata,
-- and provider external ids (mirrors the ExternalPersonID proto convention).
CREATE TYPE media_kind AS ENUM ('movie', 'episode');
ALTER TABLE media
  ADD COLUMN kind media_kind NOT NULL DEFAULT 'movie',
  ADD COLUMN source_hash varchar(64),
  ADD COLUMN duration_ms bigint,
  ADD COLUMN release_year smallint,
  ADD COLUMN season_number integer,
  ADD COLUMN episode_number integer,
  ADD COLUMN original_path varchar(1024);
-- NULLs are distinct, so legacy rows without a hash are fine.
ALTER TABLE media ADD CONSTRAINT media_source_hash_key UNIQUE (source_hash);
ALTER TABLE media_track
  ADD COLUMN track_index integer NOT NULL DEFAULT 0,
  ADD COLUMN codec varchar(32),
  ADD COLUMN is_default boolean NOT NULL DEFAULT false,
  ADD COLUMN is_forced boolean NOT NULL DEFAULT false,
  ADD COLUMN channels smallint,
  ADD COLUMN width integer,
  ADD COLUMN height integer,
  ADD COLUMN title varchar(256);
CREATE TABLE media_external_ids (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  provider varchar(64) NOT NULL,
  value varchar(256) NOT NULL,
  UNIQUE (provider, value)
);
CREATE INDEX media_external_ids_media_id_idx ON media_external_ids (media_id);
