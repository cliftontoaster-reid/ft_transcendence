# Architecture

## Overview

Dionysus is organized as a small media platform:

```text
Browser
  |
  v
SvelteKit frontend (svelte/)
  |             \
  |              \ generated protobuf clients
  v
Rust services (server/)
  |       |          |
  |       |          +-- FFmpeg encoder worker (gRPC)
  |       +------------- OIDC login service
  +--------------------- media server (DASH)
  |
  +-- PostgreSQL
  +-- Redis (OIDC login transactions)
```

The frontend is a SvelteKit application built with Vite and served by the
Node adapter. The Rust workspace contains independent crates rather than one
monolithic HTTP server. Shared wire contracts live in `proto/` and are
generated during builds.

## Repository layout

| Path | Purpose |
| --- | --- |
| `svelte/` | SvelteKit frontend, routes, components, translations, and tests |
| `server/` | Rust workspace, database crate, media services, and migrations |
| `proto/` | Protobuf contracts for users, media, rooms, playback, and encoding |
| `docs/` | Project documentation and database diagrams |
| `.devcontainer/` | Development container and PostgreSQL compose definition |
| `third_party/` | Vendored external protocol definitions and supporting files |

Generated and build output such as `svelte/build`, `svelte/.svelte-kit`, and
`server/target` should not be edited by hand.

## Service boundaries

### Frontend

The frontend owns navigation, presentation, localization, browser-side media
interaction, and calls to backend APIs. The health endpoint is exposed at
`/health`. Protobuf-generated TypeScript modules are placed below
`svelte/src/lib/proto`.

### Media server

`dionysus_media_server` is a Rocket service that serves an MPEG-DASH manifest
and video/audio segments. It reads media from `MEDIA_ROOT`, stores generated
segments in a bounded cache, and delegates transcoding to the encoder service.

### Encoder

`dionysus_ffmpeg` is a tonic gRPC service. It accepts encoding jobs and uses
FFmpeg bindings to produce media output. `MAX_CONCURRENT` limits the number of
simultaneous jobs and `LISTEN_ADDR` controls the gRPC listener.

### OIDC service

`dionysus_oidc` exposes login and callback routes. It performs provider
discovery, PKCE, state/nonce validation, and ID-token verification. Redis stores
short-lived login transactions so callback requests can be validated once.

### Database

`dionysus_db` contains Diesel models, schema declarations, and shared database
types. SQL migrations in `server/migrations` are the source of truth for the
deployed schema.
