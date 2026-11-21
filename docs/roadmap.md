# Roadmap

A short public-facing view of where the gateway is going. Task-level detail, estimates, and acceptance criteria live in [sprint-planning.md](sprint-planning.md); release notes live in [CHANGELOG.md](../CHANGELOG.md). There are no promised dates—versions are checkpoints.

## 0.1.0 — MVP

One project, one stable alias, two providers, and usage in SQL. A client can change only `base_url` and `api_key` and get completions, streaming, and cost tracking. The React admin app is scaffolded with a mock API layer but is not yet the operator interface.

## 0.2.0 — V1

Aliases with fallbacks and pricing, retries, rate limits and budgets, virtual keys with revocation, the admin API, and the React admin web app wired to a generated OpenAPI client. Packaged as one container with TLS termination at Caddy and a documented backup/restore path. This is the first version intended for real projects.

## 0.3.0 — V2

Prometheus metrics, OpenTelemetry traces, exact-match caching, guardrails with redaction defaults, key rotation with an overlap window, and richer audit views. Focus: operate the gateway with confidence and reduce repeat spend without changing the client contract.

## 0.4.0 — V3

Native `/v1/messages` and `/v1/embeddings`, A/B and rule-based routing, Postgres as an optional backend, multi-instance support, and semantic caching. Focus: scale beyond a single instance without leaking complexity into clients.

## Status

Design documentation is complete: product, architecture, data model, API contracts, security, operations, and test planning. Implementation has not started; the sprint plan is the current work breakdown.
