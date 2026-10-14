# Frontend

The frontend is a SvelteKit application in `svelte/`. It uses Svelte 5,
TypeScript, Vite, Sass, Paraglide for localization, Vitest for unit tests, and
Playwright for browser tests.

## Structure

| Path | Responsibility |
| --- | --- |
| `src/routes/` | URL routes and server endpoints |
| `src/lib/components/` | Reusable UI components |
| `src/lib/assets/` | Static component assets |
| `src/lib/paraglide/` | Generated localization runtime and messages |
| `src/lib/proto/` | Generated TypeScript protobuf modules |
| `static/` | Public static files, when present |

Routes currently include the home page, demo pages, color examples, movie
examples, and `/health`. Demo routes are useful for visual development and
should not be treated as production features without checking their status.

## Development conventions

- Keep route-specific code in its route directory.
- Put reusable UI in `src/lib/components`.
- Prefer typed props and explicit event/state types.
- Keep user-facing strings in the localization source rather than hardcoding
  them in components.
- Do not hand-edit generated files below `src/lib/paraglide` or generated
  protobuf modules.

## Generated protobuf clients

The Svelte Docker build generates TypeScript clients from `proto/` with Buf.
When changing a `.proto` file, regenerate the clients through the project
build workflow and review the generated diff together with the source change.

## Production build

`svelte.Dockerfile` builds the application with Bun and serves the SvelteKit
Node adapter on `0.0.0.0:3000`. The container health check requests
`/health`.
