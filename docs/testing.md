# Testing and quality

Run the smallest relevant check during development, then the complete checks
before submitting a cross-cutting change.

## Frontend

```sh
cd svelte
bun run check
bun run lint
bun run test:unit -- --run
bun run test:e2e
bun run build
```

Playwright may install browser binaries the first time `test:e2e` runs. Keep
browser tests focused on user-visible behavior; use Vitest for pure functions
and component-level logic.

## Rust

```sh
cd server
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features
```

OIDC integration tests that exercise Redis need a reachable Redis instance.
Tests should clean up temporary keys and avoid sharing mutable state between
cases.

## Database

For migration changes, validate:

1. Applying all migrations to a fresh database.
2. Applying the new migration to a database at the previous migration.
3. Reverting the new migration where a down migration is provided.
4. Updating the Diesel schema and DBML representation.

## Review expectations

Tests should cover input validation, failure paths, and boundary behavior, not
only the happy path. New endpoints should include a test for malformed input
and an authorization or identity check where applicable.
