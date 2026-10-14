# Development setup

## Prerequisites

The recommended environment is the repository dev container. It provides:

- Bun for frontend dependencies and scripts.
- Stable Rust, Cargo, rustfmt, Clippy, and rust-analyzer.
- `protoc` and Buf for protobuf generation.
- PostgreSQL client tools.
- A PostgreSQL 16 service.

Redis is required by the OIDC service. Install or run Redis locally when
working on authentication tests.

## Dev container

Open the repository in a Dev Container. The compose file starts a
`devcontainer` service and a PostgreSQL 16 service. PostgreSQL is available
inside the container as `postgres` and uses these defaults:

```text
database: transcendence
user:     transcendence
password: transcendence
```

The values can be overridden with `POSTGRES_DB`, `POSTGRES_USER`, and
`POSTGRES_PASSWORD`.

## Frontend workflow

```sh
cd svelte
bun install
bun run dev
```

Useful commands:

```sh
bun run check       # Svelte and TypeScript diagnostics
bun run lint        # Prettier check and ESLint
bun run test:unit   # Vitest
bun run test:e2e    # Playwright
bun run build       # Production build
```

The development server normally runs on Vite's default port. Use the URL
printed by the command rather than assuming a port when multiple services are
running.

## Rust workflow

```sh
cd server
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
```

Run a particular crate with `cargo run -p <package-name>`. For example:

```sh
cargo run -p dionysus_ffmpeg
cargo run -p dionysus_media_server
cargo run -p dionysus_oidc
```

## Database migrations

From `server/`, set `DATABASE_URL` to the PostgreSQL instance and use Diesel
CLI to run migrations:

```sh
diesel migration run
```

Do not edit an already-applied migration. Add a new migration with a clear
timestamped name and include both `up.sql` and `down.sql`.
