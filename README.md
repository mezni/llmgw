# LLM Gateway

Self-hosted, OpenAI-compatible LLM gateway written in Rust, with a React admin app. Provider keys stay in the gateway; projects get revocable virtual keys, aliases, limits, retries, and per-request usage and cost tracking.

> **Status: design phase.** This repository currently contains the product, architecture, and sprint documents. Implementation lands in S1–S7; the commands below describe the target setup.

## Documentation

- [Product document](docs/product.md) — goals, features, scope, and roadmap.
- [Architecture](docs/architecture.md) — crates, request lifecycle, data model, error model, and deployment.
- [Sprint planning](docs/sprint-planning.md) — single S1–S17 plan for MVP through V3.
- [Conventions](docs/conventions.md) — Rust and React standards, error handling, naming, testing, commits, and definition of done.
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
| MVP | S3 | One project, own key, two providers, usage in SQL |
| V1 | S7 | Aliases, pricing, resilience, limits, admin API and React app, packaging |
| V2 | S11 | Metrics/tracing, exact caching, guardrails, key rotation |
| V3 | S17 | New endpoints, smart routing, Postgres, multi-instance, semantic caching |

## Prerequisites

- Rust stable with `rustfmt` and `clippy`, plus `sqlx-cli` for migrations.
- Node 20+ with npm.
- Docker and docker-compose; Caddy terminates TLS in front of the gateway.

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
| `npm run typecheck && npm run lint && npm test` | Frontend checks (once S6 lands) |
| `npm run e2e` | Playwright smoke flows (once S6 lands) |

## Contributing

Use short-lived branches and conventional commits; see [conventions](docs/conventions.md) for commit and branch style, coding rules, testing rules, and the definition of done.
