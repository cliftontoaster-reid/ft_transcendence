-- People and their involvement in media, mirroring person.proto / credit.proto.
-- The metadata registry already merges credits but the importer never persisted
-- them; these tables give recommendations a cast/crew signal.
CREATE TABLE person (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  display_name varchar(256) NOT NULL,
  given_name varchar(256),
  family_name varchar(256),
  birth_place varchar(256),
  biography varchar(4000),
  birth_date date,
  death_date date,
  profile_picture_url varchar(2048)
);

CREATE TABLE person_external_ids (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  person_id uuid NOT NULL REFERENCES person(id) ON DELETE CASCADE,
  provider varchar(64) NOT NULL,
  value varchar(256) NOT NULL,
  UNIQUE (provider, value)
);

CREATE INDEX person_external_ids_person_id_idx ON person_external_ids (person_id);

CREATE TABLE media_credit (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  person_id uuid NOT NULL REFERENCES person(id) ON DELETE CASCADE,
  -- CreditRole values from dionysus_metadata_model, e.g. {"ACTOR","DIRECTOR"}.
  roles text[] NOT NULL DEFAULT '{}',
  characters text[] NOT NULL DEFAULT '{}',
  -- Lower = billed first; 0 when unknown.
  billing_order integer NOT NULL DEFAULT 0
);

CREATE INDEX media_credit_media_id_idx ON media_credit (media_id);
CREATE INDEX media_credit_person_id_idx ON media_credit (person_id);
