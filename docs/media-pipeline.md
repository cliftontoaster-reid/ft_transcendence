# Media pipeline

The media path is designed around on-demand MPEG-DASH segment generation.
Source files live under `MEDIA_ROOT`; the media server exposes a manifest and
requests missing segments from the encoder.

## Request flow

```text
Client requests manifest
  -> media server builds the DASH representation
Client requests segment
  -> media server checks CACHE_DIR
  -> fresh cached segment is returned, or
  -> encoder gRPC job is submitted
  -> generated segment is cached and returned
```

The default segment duration is four seconds. The server advertises a bitrate
and codec ladder, with 480p, 720p, and 1080p video defaults and AAC audio.

## Cache behavior

The segment cache is bounded by `CACHE_MAX_BYTES`. Entries older than
`CACHE_TTL_SECS` are treated as stale and can be re-transcoded. When the cache
exceeds its limit, the oldest files are evicted first.

Cache contents are disposable. They should not be committed, backed up as
source media, or used as the canonical media store.

## Encoder contract

The encoder is a tonic gRPC server defined by
`proto/dionysus/encoder/v1/job.proto`. The media server connects through
`ENCODER_ADDR`. Keep both services on compatible versions of the protobuf
contract when deploying them independently.

`MAX_CONCURRENT` provides a back-pressure limit. Increase it only after
checking CPU, memory, and disk throughput; transcoding is resource-intensive.

## Adding a format

When adding a codec, rendition, or container:

1. Update the media server configuration and manifest generation.
2. Update the encoder job handling and validate the FFmpeg arguments.
3. Add a representative source fixture or integration test.
4. Check that the browser can select and play the representation.
5. Document the new environment value and operational trade-offs.
