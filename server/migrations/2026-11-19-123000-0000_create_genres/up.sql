-- Genres improve cold-start recommendations and let ranking filter or boost by
-- genre. Populated from metadata providers during enrichment.
CREATE TABLE genre (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name varchar(64) NOT NULL UNIQUE
);

CREATE TABLE media_genre (
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  genre_id uuid NOT NULL REFERENCES genre(id) ON DELETE CASCADE,
  PRIMARY KEY (media_id, genre_id)
);

CREATE INDEX media_genre_genre_id_idx ON media_genre (genre_id);
