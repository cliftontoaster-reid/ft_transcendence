-- Cached, per-profile suggestions produced by the recommender worker and
-- served read-only by the recommendation API. Keyed on the viewer profile so
-- each profile gets its own row set; `reason` lets one profile hold several
-- overlapping lists (trending, because-you-watched, similar, friends).
CREATE TABLE recommendation (
  profile_id uuid NOT NULL REFERENCES viewer_profiles(id) ON DELETE CASCADE,
  media_id uuid NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  score real NOT NULL,
  reason varchar(64) NOT NULL,
  source_media_id uuid REFERENCES media(id) ON DELETE SET NULL,
  algorithm varchar(32) NOT NULL,
  generated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_at timestamptz,
  PRIMARY KEY (profile_id, media_id, reason)
);

CREATE INDEX recommendation_profile_score_idx
  ON recommendation (profile_id, score DESC);
