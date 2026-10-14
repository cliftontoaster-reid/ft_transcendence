# Dionysus documentation

This directory contains the project documentation for the Ft Transcendence
media platform. Start with the guides below, then use the reference pages when
working on a specific service.

## Start here

- [Architecture](architecture.md) - repository layout and service boundaries.
- [Development setup](development.md) - prerequisites and local workflows.
- [Configuration](configuration.md) - environment variables and service config.

## Components

- [Frontend](frontend.md) - SvelteKit application structure and commands.
- [Backend services](backend.md) - Rust crates and how they fit together.
- [Media pipeline](media-pipeline.md) - manifests, segments, caching, and FFmpeg.
- [Authentication](authentication.md) - OIDC login flow and Redis state.
- [Database](database.md) - PostgreSQL schema and migration workflow.
- [Protocol API](protocol-api.md) - protobuf packages and generated clients.
- [Testing and quality](testing.md) - checks to run before submitting changes.
- [Contributing](contributing.md) - conventions for changes and pull requests.

The database diagrams are kept in [`db/`](db/):

- [`db.dbml`](db/db.dbml) is the editable DBML source.
- [`db.dbdiagram`](db/db.dbdiagram) is the diagram export.

## Project status

The repository is under active development. Some protocol packages and schema
tables describe planned or partially implemented features. Treat the source
code and migrations as authoritative when a document and implementation differ.

