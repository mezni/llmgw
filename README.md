# LLM Gateway

Self-hosted, OpenAI-compatible LLM gateway written in Rust, with a React admin app. Provider keys stay in the gateway; projects get revocable virtual keys, aliases, limits, retries, and per-request usage and cost tracking.

> **Status: S2 streaming.** The OpenAI-compatible data plane supports non-streaming and SSE streaming completions with client-disconnect cancellation. Virtual-key auth (deferred to S3), usage logging, and the admin API/UI land later; the target setup below describes the finished system.

## Documentation

- [Product document](docs/product.md) — goals, features, scope, and roadmap.
- [Public roadmap](docs/roadmap.md) — short version-oriented view of the sprint plans.
- [Architecture](docs/architecture.md) — crates, request lifecycle, data model, error model, and deployment.
- [Data-plane API spec](docs/api-spec.md) — endpoints, fields, streaming, headers, and error contract.
- [Admin API](docs/admin-api.md) and [OpenAPI spec](docs/openapi.json) — control-plane contract and generated client input.
- [Admin UI](docs/admin-ui.md) — pages, routes, components, state, auth, and design basics.
- [Data model](docs/data-model.md) — schema, migrations, indexes, and retention.
- [Config reference](docs/config-reference.md) — every env var and TOML setting, default, and restart flag.
- [Provider compatibility](docs/provider-compatibility.md) — per-provider feature matrix and known quirks.
- [Limitations](docs/limitations.md) — deliberate non-goals and versioned gaps.
- [Security](docs/security.md) — threat model, key handling, encryption, and logging rules.
- [Test plan](docs/test-plan.md) — test levels, mock scenarios, fixtures, and CI gates.
- [Operations](docs/operations.md) — deploy, backup/restore, key rotation, upgrades, and failures.
- [Observability](docs/observability.md) — log fields, metrics, dashboards, and alerts.
- [Glossary](docs/glossary.md) — terms shared by docs and code.
- [Sprint planning](docs/sprint-planning.md) — single S1–S17 plan for MVP through V3.
- [Conventions](docs/conventions.md) — Rust and React standards, error handling, naming, testing, commits, and definition of done.
- [Architecture decision records](docs/adr/) — one short record per significant choice.
- [Changelog](CHANGELOG.md) — notable changes and planned releases.

## Stack

| Area | Choice |
|------|--------|
| Language / HTTP | Rust, axum on tokio |
| Upstream client | reqwest (streaming) |
| Storage | SQLite via sqlx, WAL mode |
| Config | TOML + environment overrides |
| Data plane | `POST /v1/chat/completions`, streaming SSE, OpenAI-compatible errors |
| Admin API | `/admin/v1`, OpenAPI via `utoipa`, session cookie auth |
| Admin UI | Vite + React + TypeScript, TanStack Router/Query, shadcn/ui + Tailwind, React Hook Form + Zod, Recharts, TanStack Table |
| Tests | `cargo test`, Vitest + React Testing Library, Playwright |
| Packaging | Multi-stage Docker image, Caddy for TLS |

## Repository layout

```
gateway/
├── crates/
│   ├── core/        # domain types and errors, no I/O
│   ├── providers/   # Provider trait and adapters
│   ├── storage/     # sqlx models, repositories, migrations
│   ├── gateway/     # data plane: routes, middleware, routing engine
│   ├── admin-api/   # control plane: routes, admin auth
│   └── server/      # binary: config, wiring, startup, shutdown
├── admin-ui/        # React admin app
├── tests/           # integration tests + mock provider
├── docs/            # product, architecture, sprint plan, conventions
├── docker/          # Dockerfile, compose, Caddy
└── .env.example     # planned: local configuration template
```

Dependency direction is one-way: `server → gateway/providers/admin-api → storage → core`. See [architecture §2](docs/architecture.md).

## Milestones

| Checkpoint | Sprint | Scope |
|------------|--------|-------|
| MVP | S3 | One project, own key, two providers, usage in SQL, React app scaffold |
| V1 | S7 | Aliases, pricing, resilience, limits, admin API and React app, packaging |
| V2 | S11 | Metrics/tracing, exact caching, guardrails, key rotation |
| V3 | S17 | New endpoints, smart routing, Postgres, multi-instance, semantic caching |

## Prerequisites

- Rust stable with `rustfmt` and `clippy`, plus `sqlx-cli` for migrations.
- Node 20+ with npm.
- Docker and docker-compose; Caddy terminates TLS in front of the gateway.

## Development (S1–S2)

```bash
cargo build
cargo test
cargo run -p server
```

With no `GATEWAY_UPSTREAM_BASE_URL` set, the server starts an in-process mock provider and the data plane accepts normal and streaming Chat Completions requests:

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"fast","messages":[{"role":"user","content":"Hello"}]}'

curl -N http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"fast","messages":[{"role":"user","content":"Hello"}],"stream":true}'
```

Point at a real OpenAI-compatible upstream with `GATEWAY_UPSTREAM_BASE_URL` and `GATEWAY_UPSTREAM_API_KEY`. There is no virtual-key auth yet (deferred to S3), so do not expose the listener publicly. `GET /health` returns `{"status":"ok"}` and every response carries `X-Request-Id`. Streaming responses are OpenAI-compatible SSE chunks ending in `[DONE]`; client disconnects cancel the upstream request.

## Development (target)

```bash
export DATABASE_URL=sqlite://./gateway.db
export GATEWAY_MASTER_KEY=<32-byte key>
export ADMIN_BOOTSTRAP_EMAIL=admin@example.com
export ADMIN_BOOTSTRAP_PASSWORD=<strong password>
export RUST_LOG=info

sqlx migrate run
cargo build
cargo test
cargo run -p server
```

The data plane listens on `:8080` and the admin API/UI on `:8081`. For UI development, run `npm install && npm run dev` in `admin-ui/` with the dev proxy to `:8081`; the built app is served from the admin listener in production.

Once a key is issued, a client only changes `base_url` and `api_key`:

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer gw-..." \
  -H "Content-Type: application/json" \
  -d '{"model":"smart","messages":[{"role":"user","content":"Hello"}]}'
```

## Verification

| Command | Purpose |
|---------|---------|
| `cargo fmt --all -- --check` | Rust formatting |
| `cargo clippy --all-targets -- -D warnings` | Rust lints |
| `cargo test` | Unit, adapter, integration, and admin tests |
| `npm run typecheck && npm run lint && npm test` | Frontend checks (once the S1 scaffold lands) |
| `npm run e2e` | Playwright smoke flows (once S6 lands) |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow and [conventions](docs/conventions.md) for commit and branch style, coding rules, testing rules, and the definition of done.
