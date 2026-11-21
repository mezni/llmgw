# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] — targeting 0.1.0

### Added

- Rust workspace with `core`, `providers`, `storage`, `gateway`, and `server` crates following the documented dependency direction.
- Axum data plane with `GET /health`, JSON tracing logs, request IDs on responses and log lines, and configurable TOML/environment settings.
- Normalized chat types with serde fixtures, an OpenAI-compatible adapter, and an in-process mock provider with `ok`, `delay`, `429`, and `500` scenarios.
- `POST /v1/chat/completions` with OpenAI-shaped errors and an end-to-end smoke test using the official Python SDK.
- OpenAI-compatible SSE streaming for chat completions: normalized chunks, fragmented-frame parsing, `[DONE]` terminator, final error events, and upstream cancellation on client disconnect.
- Mock streaming scenarios (`stream_ok`, `stream_cut`, `stream_malformed`, `client_cancel`) and OpenAI stream-chunk fixtures.
- Product document: goals, features, scope, users, and roadmap.
- Architecture document: crates, request lifecycle, usage logging, data model, error model, security, testing, and deployment.
- Data-plane API specification: endpoints, supported fields, pass-through rules, SSE format, headers, and error contract.
- API, data-model, admin API/OpenAPI, admin UI, provider compatibility, and security documents.
- Config reference, test plan, operations runbook, observability, glossary, roadmap, and limitations.
- Sprint plan: single S1–S17 plan covering MVP, V1, V2, and V3, including the incremental React admin app.
- Conventions: Rust and React coding standards, error handling, module layout, naming, testing, commits, and definition of done.
- Architecture decision records for SQLite, axum, plane separation, aliases, and the React admin UI.
- README, CONTRIBUTING, and changelog.

### Changed

- `stream: true` is now served as SSE instead of being rejected; virtual-key auth remains deferred to S3.
- README status and development examples now cover streaming.
- Aligned audit logging to V1 and key rotation to V2.
- Made the OpenAPI spec and generated client a hard dependency for the admin UI sprint.
- Moved spend accounting off the droppable usage-log path so budget enforcement stays correct under load.
- Scaffolded the React admin UI in S1 (MVP) and deferred generated-client wiring to S6.
- Switched release numbering to 0.1.0 → 0.2.0 → 0.3.0 → 0.4.0; removed the 1.0.0/2.0.0/3.0.0 milestones.

## Planned releases

| Version | Target | Contents |
|---------|--------|----------|
| 0.1.0 | End of S3 | MVP: passthrough, streaming, virtual keys, usage logging, Anthropic adapter, React admin UI scaffold |
| 0.2.0 | End of S7 | V1: aliases, pricing, retries/fallbacks, limits and budgets, admin API, React admin app, packaging |
| 0.3.0 | End of S11 | V2: Prometheus metrics, OpenTelemetry traces, exact-match caching, guardrails, key rotation, audit views |
| 0.4.0 | End of S17 | V3: `/v1/messages`, embeddings, smart routing, Postgres, multi-instance, semantic caching |

Versions increment as 0.1.0 → 0.2.0 → 0.3.0 → 0.4.0; there is no 1.0.0 milestone in this plan. Planned scope is tracked in [sprint-planning.md](docs/sprint-planning.md); release notes are written when each version is tagged.
