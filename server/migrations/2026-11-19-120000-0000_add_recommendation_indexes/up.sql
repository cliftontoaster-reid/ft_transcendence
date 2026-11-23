-- Indexes the recommendation queries rely on. Both are described in
-- docs/db/db.dbml but no earlier migration created them: the full-text index
-- backs keyword fallbacks, the HNSW index backs embedding similarity.
CREATE INDEX IF NOT EXISTS media_translations_fts
  ON media_translations USING gin (fts_content);

CREATE INDEX IF NOT EXISTS media_translations_embedding_hnsw
  ON media_translations USING hnsw (embedding vector_cosine_ops);
