# Status

## Current sprint: S1 — Foundations (MVP)
- Branch: main (sprints run directly on main per MASTER_PROMPT.md)
- Started / updated: 2026-11-20
- State: complete

Committed scope: 18 pts (1.1, 1.2, 1.3, 1.5, 1.6, 1.7, 1.8). The sprint table lists 25 pts; capacity is ~16 and 18 is the smallest set that delivers the end-to-end demo. Deferred: 1.4 (CI — sprint note says drop to S2), 1.9 (full upstream error-matrix mapping), 1.10 (UI scaffold). None are "Never cut" items.

### Task plan
| Id | Task | Pts | State | Evidence |
|----|------|-----|-------|----------|
| 1.1 | Create Cargo workspace (core, providers, storage, gateway, server) | 2 | done | `cargo build` passes; one-way dependency graph |
| 1.2 | axum server with `/health` and tracing JSON logs | 2 | done | integration test `health_returns_ok`; request id on response and log line |
| 1.3 | Config loading (TOML + env overrides) | 2 | done | 5 config unit tests cover defaults, TOML, env precedence, validation |
| 1.5 | Normalized types in core with serde | 3 | done | 3 fixture round-trip/rejection tests |
| 1.6 | Mock provider server (ok, delay, 429, 500) | 3 | done | 4 adapter tests exercise ok/429/500/400; delay supported |
| 1.7 | Provider trait + OpenAI-compatible adapter (non-streaming) | 3 | done | adapter tests classify 429, 500, 400; no credential echo |
| 1.8 | `POST /v1/chat/completions` handler wired to the adapter | 3 | done | 5 gateway integration tests + OpenAI Python SDK smoke |
| 1.4 | CI: fmt, clippy, test on push | 2 | deferred | Sprint note: drop to S2 if needed |
| 1.9 | OpenAI-style error mapping (full 400/429/500 matrix) | 2 | deferred | Minimal OpenAI-shaped errors included in 1.8 |
| 1.10 | Vite + React + TS admin UI scaffold | 3 | deferred | Stretch; backend-first vertical slice |

### Detailed task cards

#### 1.1 — Cargo workspace
- Acceptance: `cargo build` passes; crates follow the dependency direction in architecture.md §2.
- Depends on: none. Files: `Cargo.toml`, `crates/{core,providers,storage,gateway,server}/Cargo.toml`, `lib.rs`/`main.rs`, `.gitignore`.
- Tests: `cargo build`; workspace dependency graph has no cycles.
- Docs: README layout and CHANGELOG updated.

#### 1.2 — axum server, `/health`, JSON logs
- Acceptance: `GET /health` returns 200 `{"status":"ok"}`; each request logs a request ID.
- Depends on: 1.1. Files: `crates/server/src/main.rs`, `crates/gateway/src/router.rs`, `crates/gateway/src/request_id.rs`.
- Tests: integration test for 200 body and `X-Request-Id`; supplied ID echoed, absent ID generated; JSON log line contains `request_id`.
- Docs: api-spec.md §2/§3 already define the contract; no change needed.

#### 1.3 — Config loading
- Acceptance: listen address and log level configurable without recompiling; env → TOML → default precedence.
- Depends on: 1.1. Files: `crates/server/src/config.rs`, `config.example.toml`.
- Tests: unit tests for defaults, precedence, malformed values, and validation.
- Docs: config-reference.md §1 updated with the S1 upstream variables.

#### 1.5 — Core normalized types
- Acceptance: round-trips a real OpenAI request and response fixture.
- Depends on: 1.1. Files: `crates/core/src/chat.rs`, `crates/core/src/error.rs`, `tests/fixtures/providers/openai/*`.
- Tests: serde round-trip of request and response fixtures; rejects multimodal content.
- Docs: fixtures are scrubbed; api-spec.md unchanged.

#### 1.6 — Mock provider
- Acceptance: configurable `ok`, `delay`, `429`, `500`; selected per request via header or request field; usable by integration tests.
- Depends on: 1.5. Files: `crates/providers/src/mock.rs`.
- Tests: 4 adapter tests plus 5 gateway integration tests.
- Docs: test-plan.md §2 scenario names unchanged.

#### 1.7 — Provider trait + OpenAI adapter
- Acceptance: adapter passes tests against the mock provider for non-streaming chat.
- Depends on: 1.5, 1.6. Files: `crates/providers/src/lib.rs`, `error.rs`, `openai.rs`.
- Tests: request/response mapping, error classification, retryability flag.
- Docs: provider-compatibility.md remains accurate for the implemented subset.

#### 1.8 — Chat completions handler
- Acceptance: end-to-end works with the OpenAI Python SDK against the gateway.
- Depends on: 1.2, 1.7. Files: `crates/gateway/src/routes/chat.rs`, `error.rs`, `crates/server/src/main.rs`.
- Tests: integration tests for success, request id, validation, upstream errors; SDK smoke with openai 3.24.0.
- Docs: README S1 development section and CHANGELOG updated.

### Verification
| Check | Command | Result |
|-------|---------|--------|
| Format | `cargo fmt --all -- --check` | pass |
| Lint | `cargo clippy --offline --all-targets -- -D warnings` | pass |
| Rust tests | `cargo test --offline` | pass — 20 tests (6 core, 4 adapter, 5 integration, 5 config) |
| Build | `cargo build --offline` | pass |
| SDK smoke | OpenAI Python SDK 3.24.0 against `http://127.0.0.1:18080/v1` | pass — completion id/model/content/finish_reason/usage match |
| Frontend | `npm run typecheck && npm run lint && npm test` | not applicable — 1.10 deferred, no UI tree |
| OpenAPI drift | `git diff --exit-code -- docs/openapi.json ...` | not applicable — no control-plane change |
| Migration | `sqlx migrate run` | not applicable — no migrations in S1 scope |
| E2E | `npm run e2e` | not applicable — S6 onward |
| Security | `cargo audit`, `cargo deny`, secret scan | not applicable — S17/release gate; no secrets added |

### Decisions and deviations
- Sprints run directly on `main` (MASTER_PROMPT.md override); no feature branch.
- Committed 18 pts rather than exactly 16: dropping more would break the end-to-end demo (1.6 and 1.8 are both required by the goal).
- The core crate is packaged as `gateway-core` in `crates/core/` to avoid colliding with Rust's built-in `core` crate.
- The mock accepts `x-mock-scenario` from a header or the JSON body so tests can select scenarios without a separate client.
- Upstream selection is env-only in S1 (`GATEWAY_UPSTREAM_BASE_URL`, `GATEWAY_UPSTREAM_API_KEY`); documented in config-reference.md.
- `stream: true` is rejected with `unsupported_field` until S2 implements SSE.
- Unknown TOML keys are ignored in S1; config-reference.md specifies a warning, to be aligned in a later sprint.

### Deferred / carryover
- 1.4 — CI pipeline; sprint-planning.md S1 note says drop to S2 if needed.
- 1.9 — full upstream error matrix; minimal OpenAI-shaped errors are included in 1.8.
- 1.10 — UI scaffold; stretch item, backend-first vertical slice.

### Blockers
- None.

### Definition of done
- [x] Acceptance criteria met
- [x] fmt, clippy, tests pass
- [x] Mock-provider scenario covered
- [x] Docs and CHANGELOG updated
- [x] No secrets in logs, fixtures, or repo
- [x] Committed on main and pushed

## Sprint history
| Sprint | Version | State | Commit / tag | Notes |
|--------|---------|-------|--------------|-------|
| S1 | MVP | complete | pending | Foundations: data plane vertical slice |
