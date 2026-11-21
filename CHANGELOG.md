# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Product document: goals, features, scope, users, and roadmap.
- Architecture document: crates, request lifecycle, usage logging, data model, error model, security, testing, and deployment.
- Data-plane API specification: endpoints, supported fields, pass-through rules, SSE format, headers, and error contract.
- Provider compatibility matrix and security model documents.
- Sprint plan: single S1–S17 plan covering MVP, V1, V2, and V3, including the incremental React admin app.
- Conventions: Rust and React coding standards, error handling, module layout, naming, testing, commits, and definition of done.
- Architecture decision records for SQLite, axum, plane separation, aliases, and the React admin UI.
- README and changelog.

### Changed

- Aligned audit logging to V1 and key rotation to V2.
- Made the OpenAPI spec and generated client a hard dependency for the admin UI sprint.
- Moved spend accounting off the droppable usage-log path so budget enforcement stays correct under load.
- Scaffolded the React admin UI in S1 (MVP) and deferred generated-client wiring to S6.

## Planned releases

| Version | Target | Contents |
|---------|--------|----------|
| 1.0.0 | End of S7 | MVP + V1: passthrough, streaming, virtual keys, usage logging, Anthropic adapter, aliases, retries/fallbacks, limits, admin API, React admin app, packaging |
| 2.0.0 | End of S11 | V2: Prometheus metrics, OpenTelemetry traces, exact-match caching, guardrails, key rotation, audit views |
| 3.0.0 | End of S17 | V3: `/v1/messages`, embeddings, smart routing, Postgres, multi-instance, semantic caching |

Planned scope is tracked in [sprint-planning.md](docs/sprint-planning.md); release notes are written when each version is tagged.
