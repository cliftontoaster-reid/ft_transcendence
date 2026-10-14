# Database

PostgreSQL stores users, authentication identities, media metadata, viewer
profiles, playback state, and room participation. The schema is represented
in three related places:

- SQL migrations in `server/migrations/` - deployment source of truth.
- Diesel declarations in `server/crates/dionysus_db/src/schema.rs`.
- DBML diagrams in `docs/db/`.

## Main domains

### Media

`media` is the canonical media record. Localized metadata lives in
`media_translations`; images and playable tracks are represented by
`media_images` and `media_track`.

### Accounts

`users` represents a local account and `oidc_identities` links it to an OIDC
issuer/subject pair. Roles and permissions are modeled through
`user_roles` and `role_permissions`.

### Viewer state

`viewer_profiles` supports multiple viewing profiles per account.
`watch_progress`, `watch_history`, and `my_list` are keyed by profile so
playback and library state remain profile-specific.

### Rooms

`rooms` and `room_participents` model shared viewing or collaboration rooms.
The participant role enum distinguishes owners, administrators, and users.

## Migration workflow

Create a new migration for every schema change. Keep migrations reversible
where practical:

```sh
cd server
diesel migration generate describe_the_change
diesel migration run
diesel migration revert
```

After changing the schema, update Diesel's generated or maintained schema
declarations and the DBML diagram. Test both a fresh database migration and an
upgrade from the latest existing migration.

## Data guidelines

- Use UUIDs for externally visible identifiers.
- Preserve foreign-key relationships.
- Add indexes for lookup paths used by API requests.
- Keep timestamps timezone-aware.
- Never store provider secrets or raw authentication tokens in PostgreSQL.
