-- Explicit feedback (ratings) and implicit interaction events (impressions,
-- clicks, plays, dismissals) that feed the ranking and the feedback loop.
CREATE TABLE media_rating (
  profile_id uuid NOT NULL REFERENCES viewer_profiles(id) ON DELETE CASCADE,
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  value smallint NOT NULL CHECK (value BETWEEN 1 AND 5),
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (profile_id, media_id)
);

CREATE TABLE recommendation_event (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  profile_id uuid NOT NULL REFERENCES viewer_profiles(id) ON DELETE CASCADE,
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  -- impression | click | play | dismiss
  kind varchar(16) NOT NULL,
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX recommendation_event_profile_created_at_idx
  ON recommendation_event (profile_id, created_at DESC);
