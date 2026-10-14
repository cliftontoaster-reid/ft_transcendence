# Backend services

The Rust workspace is defined in `server/Cargo.toml` and uses Rust edition
2024. Each service has a focused responsibility.

## Crates

| Package | Role |
| --- | --- |
| `dionysus_db` | Diesel schema, models, and shared database types |
| `dionysus_ffmpeg` | tonic gRPC encoding worker backed by FFmpeg |
| `dionysus_media_server` | Rocket MPEG-DASH manifest and segment server |
| `dionysus_oidc` | Rocket OIDC login and callback service |

## Error handling

Services use typed Rust errors internally and map them to protocol-appropriate
responses at the boundary. The OIDC routes distinguish invalid client input
from provider or infrastructure failures; preserve that distinction when
adding endpoints.

## Logging

The services initialize `tracing_subscriber` with an environment filter.
Set `RUST_LOG` while developing to increase verbosity, for example:

```sh
RUST_LOG=debug cargo run -p dionysus_media_server
```

Avoid logging access tokens, client secrets, authorization codes, or complete
user credentials.

## Protobuf build

The encoder and media crates use `tonic-prost-build` in their build scripts.
The generated Rust modules are included with `tonic::include_proto!`; they are
build artifacts and should not be edited manually.

## Adding a service

1. Add a crate under `server/crates/`.
2. Register it in the workspace members list.
3. Define its configuration and listener contract.
4. Add or reuse protobuf messages in `proto/` when communication crosses a
   process boundary.
5. Document its environment variables and health behavior.
6. Add focused tests and workspace-level validation.
