# Configuration

Configuration is intentionally environment-driven for services and
file-driven for OIDC provider definitions. Never commit real credentials.

## Database

The dev container supplies `DATABASE_URL` in this form:

```text
postgres://USER:PASSWORD@postgres:5432/DATABASE
```

For a host-installed PostgreSQL instance, use `localhost` instead of
`postgres`. The checked-in `server/.env.template` is only a placeholder and
must be replaced with a real local value.

## Media server

`dionysus_media_server` reads these variables:

| Variable | Default | Meaning |
| --- | --- | --- |
| `MEDIA_ROOT` | `./media` | Source media directory |
| `CACHE_DIR` | `./cache/segments` | Generated segment cache |
| `CACHE_MAX_BYTES` | `10737418240` | Maximum cache size |
| `CACHE_TTL_SECS` | `3600` | Maximum age of a reusable segment |
| `SEGMENT_DURATION_SECS` | `4` | Segment duration |
| `ENCODER_ADDR` | `http://[::1]:50051` | Encoder gRPC endpoint |
| `VIDEO_BITRATE_LOW` | `800000` | Low video bitrate |
| `VIDEO_BITRATE_MEDIUM` | `2500000` | Medium video bitrate |
| `VIDEO_BITRATE_HIGH` | `6000000` | High video bitrate |
| `AUDIO_BITRATE_LOW` | `96000` | Low audio bitrate |
| `AUDIO_BITRATE_MEDIUM` | `160000` | Medium audio bitrate |
| `AUDIO_BITRATE_HIGH` | `320000` | High audio bitrate |
| `LADDER_RESOLUTIONS` | `480,720,1080` | Advertised video heights |
| `LADDER_VIDEO_CODECS` | `h264` | Video codec names |
| `LADDER_AUDIO_CHANNELS` | `2,6` | Audio channel counts |
| `LADDER_AUDIO_CODECS` | `aac` | Audio codec names |

List values are comma-separated. Resolution values may include a trailing
`p`, such as `480p,720p`.

## Encoder

`dionysus_ffmpeg` accepts:

| Variable | Default | Meaning |
| --- | --- | --- |
| `LISTEN_ADDR` | `[::1]:50051` | gRPC listen address |
| `MAX_CONCURRENT` | detected CPU count, clamped to 1-8 | Concurrent jobs |

Keep the encoder address and media server `ENCODER_ADDR` aligned.

## OIDC

The OIDC service reads the path in `OIDC_CONFIG`, defaulting to
`config/oidc.toml`. The file contains an `oidc` section with one or more
providers. Each provider needs an issuer URL, client ID, client secret
reference, redirect URI(s), and an `openid` scope.

`REDIS_URL` defaults to `redis://127.0.0.1:6379/`. Login transactions expire
after ten minutes and are deleted when consumed.
