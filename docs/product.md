# LLM Gateway — Product Document

## 1. Overview

A self-hosted, OpenAI-compatible LLM gateway written in Rust, with an admin web app. It sits between my projects and LLM providers (Anthropic, OpenAI, and others), giving every project one stable API, centralized keys, routing, limits, and usage tracking.

Primary purpose: learning. Building it from scratch is the point: async Rust, streaming, trait-based abstractions, middleware, and production-grade service design.
Secondary purpose: a genuinely reusable piece of infrastructure for all my own projects.

## 2. Problem

Every project that calls an LLM re-implements the same things:

- Provider keys scattered across repos and `.env` files
- Provider-specific SDKs and request formats
- No shared view of cost, latency, or errors
- Ad hoc retries, timeouts, and fallbacks
- Model names hardcoded in application code, making model swaps painful

## 3. Goals

- **One API for all projects.** OpenAI-compatible, so existing SDKs and frameworks work by changing only `base_url` and `api_key`.
- **Keys stay in one place.** Real provider keys live only in the gateway; projects get revocable virtual keys.
- **Model indirection.** Projects request aliases (`fast`, `smart`); I remap them without redeploying anything.
- **Resilience.** Retries, timeouts, and fallbacks handled centrally.
- **Visibility and control.** Per-project usage, cost, latency, rate limits, and budgets.
- **Learn deeply.** Each milestone teaches a concrete Rust or systems concept.

## 4. Non-goals

- A multi-tenant SaaS or a public product with billing
- Competing with LiteLLM, Portkey, or Cloudflare AI Gateway on breadth
- Hosting or running models itself
- A prompt-management or evaluation platform (possible later, out of scope now)
- Horizontal scaling and high availability in v1

## 5. Users

| User | Needs |
|------|-------|
| Me, as developer | Point any project at the gateway with minimal changes; swap models quickly; debug failed calls |
| Me, as operator | Create and revoke keys, set budgets, watch cost and errors, manage providers |
| My projects (clients) | A stable, fast, OpenAI-compatible endpoint with predictable errors |

## 6. Core concepts

- **Project:** a consumer of the gateway (an app or experiment).
- **Virtual key:** a secret issued to a project, stored hashed, revocable, with limits.
- **Provider:** an upstream LLM service with credentials and a base URL.
- **Model:** a concrete provider model with pricing.
- **Alias:** a stable name (`fast`, `smart`) mapped to a model, with an optional fallback chain.
- **Request log:** one record per call with tokens, cost, latency, and status.

## 7. Features

### 7.1 Data plane (what projects call)

- `POST /v1/chat/completions` with streaming (SSE)
- `GET /v1/models` listing available aliases
- `GET /health` and `GET /ready`
- Tool/function calling and structured output passthrough
- Provider-specific passthrough via an extra field
- Normalized, OpenAI-style error responses

### 7.2 Providers and routing

- Provider trait with adapters: OpenAI-compatible first, Anthropic second
- Alias resolution per request, with a per-key alias allowlist
- Retries with exponential backoff and jitter, honoring `Retry-After`
- Fallback chains (applied only before the first streamed byte)
- Per-route timeouts and a circuit breaker per provider

### 7.3 Auth, limits, and budgets

- Virtual keys per project: create, revoke, expire (rotation in V2)
- Rate limits per key (requests/min, tokens/min)
- Monthly spend cap per project with hard cut-off
- Separate admin authentication from data-plane keys

### 7.4 Usage and cost tracking

- One log row per request: project, key, model, provider, tokens in/out, latency, time to first token, status, cost
- Editable pricing table
- Optional tags via a header for per-feature breakdowns
- Configurable logging level: off, metadata only, or full prompt and response

### 7.5 Admin API

- CRUD for projects, keys, providers, aliases, pricing, and limits
- Usage queries (totals and time series by project, model, day)
- Audit log of admin mutations
- Provider credentials encrypted at rest
- OpenAPI specification

### 7.6 Admin web app

- Login
- Dashboard: requests, tokens, cost, latency, error rate
- Projects and keys (secret shown once)
- Alias and fallback editor
- Limits and budgets editor
- Provider health
- Request log viewer with filters
- Playground for testing an alias

### 7.7 Later (post-V1)

- Response caching (exact match, then semantic)
- Guardrails: PII redaction, blocked patterns
- Prometheus metrics and OpenTelemetry traces
- Native `/v1/messages` (Anthropic format) endpoint
- Embeddings endpoint
- A/B and canary routing
- Postgres support, key rotation workflows, rich audit-log views

## 8. Technical approach

| Area | Choice |
|------|--------|
| Language | Rust |
| HTTP server | axum on tokio |
| Upstream client | reqwest (streaming) |
| Serialization | serde / serde_json |
| Storage | SQLite via sqlx (Postgres later) |
| Config | TOML + environment overrides |
| Observability | tracing, later OpenTelemetry |
| Admin UI | Vite + React + TypeScript SPA, API client generated from the OpenAPI spec, shadcn/ui + Tailwind; served by the admin listener |
| Packaging | Docker, docker-compose, Caddy for TLS |

### Architecture

```
Projects ──► Data plane (/v1/*) ──► Provider adapters ──► Anthropic / OpenAI / ...
                  │
                  ├── auth, limits, routing, retries
                  └── async usage logger ──► SQLite
Admin UI ──► Admin API (separate router/port) ──► SQLite
```

### Workspace layout

```
gateway/
├── crates/
│   ├── core/        # shared types and errors
│   ├── providers/   # Provider trait and adapters
│   ├── gateway/     # data plane
│   ├── admin-api/   # control plane
│   └── storage/     # sqlx models and migrations
└── admin-ui/
```

### Key design decisions

- Data plane and control plane are separate routers with separate auth.
- Usage logging is asynchronous (channel + background writer) and never blocks or fails a request.
- Never retry or fall back after streaming has started.
- Cancel the upstream request when the client disconnects.
- Provider keys are never logged; prompt logging is configurable.

## 9. Success criteria

- An existing project works against the gateway by changing only `base_url` and `api_key`.
- Streaming works end to end with correct cancellation.
- Switching an alias to a different model needs no redeploy of any project.
- A failing provider triggers fallback and the attempt chain appears in the logs.
- A project that exceeds its budget or rate limit gets a clear 429.
- I can onboard a new project end to end from the admin app, without SQL or CLI.
- `docker compose up` gives a working setup from a fresh clone.

## 10. Roadmap

| Phase | Scope |
|-------|-------|
| MVP (M0–M5) | Workspace setup, passthrough to one provider, streaming, virtual keys, usage logging, Anthropic adapter, React admin UI scaffold |
| V1 (M6–M11) | Aliases and pricing, retries and fallbacks, rate limits and budgets, admin API, admin web app, hardening and packaging |
| V2 | Caching, guardrails, metrics and tracing, key rotation, audit-log views |
| V3 | Semantic cache, A/B routing, `/v1/messages`, embeddings, Postgres |

## 11. Risks and open questions

| Risk / question | Mitigation |
|-----------------|------------|
| Abstraction leaks across providers (tools, structured output, reasoning params) | Normalize the common subset; allow provider-specific passthrough |
| Mid-stream failures are hard to recover from | Fall back only before the first byte; surface clean stream errors |
| Cost and token accounting differs per provider | Use provider-reported usage; store raw usage alongside the computed cost |
| Scope creep from the V2/V3 list | Finish M11 before starting anything beyond V1 |
| Generated client drift from the Rust API | Regenerate the TypeScript client in CI and fail the build on drift; OpenAPI is a hard dependency for the UI sprint |
| Sensitive prompts in logs | Default to metadata-only logging; make full logging opt-in per project |

## 12. Out-of-scope reminders

No multi-tenancy, no billing, no model hosting, no horizontal scaling in v1.
