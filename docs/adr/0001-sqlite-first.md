# ADR 0001: Use SQLite first, with a path to Postgres

- Status: Accepted
- Date: 2026-11-20
- Related: ADR 0002 (axum), architecture §6 and §15, sprint plan S14

## Context

The gateway needs durable storage for config (projects, keys, providers, models, aliases) and an append-only request log. It is a self-hosted, single-instance side project whose primary purpose is learning Rust and service design. The operator wants zero-ops setup and a single container, but a later V3 milestone may need Postgres for multi-instance deployment.

## Decision

Use SQLite via `sqlx` for V1 and V2. Run in WAL mode; route all writes through bounded paths (one batched usage writer plus admin mutations) so a single writer cannot starve readers or itself. Keep SQL confined to the `storage` crate and expose repositories to the rest of the workspace. Treat V3 Postgres support as an optional backend reached through repository traits, with the schema and migration tool verified in S14.

## Alternatives considered

- **Postgres from day one:** more operational overhead (server, credentials, backups) for a single-instance learning project.
- **Embedded key/value store:** would not support the relational config and time-series queries the admin app needs.

## Consequences

- Setup is `DATABASE_URL=sqlite://...` plus `sqlx migrate run`; no external service to run.
- Backups use `sqlite3 .backup` or Litestream; a mounted volume holds the database.
- Concurrency is bounded by a single writer, which is acceptable at this scale.
- Because `sqlx` and the repository boundary stay backend-agnostic, the V3 move to Postgres does not leak into handlers or domain types.
- Multi-instance deployment is out of scope until S14–S15, where the actual requirement will be reassessed.
