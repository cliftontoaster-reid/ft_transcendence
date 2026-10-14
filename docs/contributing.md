# Contributing

## Before editing

Read the component documentation relevant to the change and inspect existing
patterns before introducing a new helper, service, or protocol message.

## Making changes

- Keep commits focused and explain the user-visible or operational impact.
- Preserve existing formatting and naming conventions.
- Update directly related documentation when behavior or configuration changes.
- Do not commit secrets, generated build output, caches, or local databases.
- Prefer typed errors and explicit validation at service boundaries.
- Keep generated files reproducible from their source definitions.

## Pull requests

A useful pull request description includes:

- what changed and why;
- affected services, routes, or schema objects;
- configuration or migration requirements;
- tests and checks run;
- known limitations or follow-up work.

For database and protobuf changes, include the migration or compatibility
implications explicitly. For media changes, include the affected codecs,
bitrates, cache behavior, and a representative playback check.

## Documentation style

Documentation belongs under `docs/` unless it is tightly coupled to a local
tool or package. Prefer short pages with concrete commands, tables for
configuration, and links to source-of-truth files. When implementation is
incomplete, say so rather than documenting an intended behavior as shipped.
