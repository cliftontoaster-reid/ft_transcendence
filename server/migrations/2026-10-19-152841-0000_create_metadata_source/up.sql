-- Registry of metadata sources (native providers and WASM plugins): the
-- descriptor each one reports, upserted whenever it contributes during
-- enrichment. Per-media linkage stays in media_external_ids.provider.
CREATE TABLE metadata_source (
  id varchar(64) PRIMARY KEY,
  name varchar(255) NOT NULL,
  version varchar(64) NOT NULL,
  model_version varchar(64) NOT NULL
);
