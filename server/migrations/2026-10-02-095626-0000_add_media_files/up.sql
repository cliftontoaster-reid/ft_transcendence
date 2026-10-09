CREATE TYPE media_track_type AS ENUM (
  'video',
  'audio',
  'subtitles'
);
CREATE TABLE IF NOT EXISTS media_track (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  track_type media_track_type NOT NULL,
  locale varchar(35) NOT NULL,
  storage_path varchar(255) NOT NULL
);
CREATE INDEX IF NOT EXISTS media_track_media_id_idx ON media_track (media_id);
