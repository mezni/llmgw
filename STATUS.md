# Status

## Current sprint: S2 — Streaming and auth (MVP)
- Branch: main (sprints run directly on main per MASTER_PROMPT.md)
- Started / updated: 2026-11-20
- State: complete

Committed scope: 16 pts (2.1, 2.2, 2.3, 2.4, 2.5). The sprint table lists 26 pts; the S2 note prioritizes 2.1–2.5 and explicitly allows 2.6–2.9 to spill to S3. Streaming correctness (2.1–2.5) is a "Never cut" item and is fully committed. Key hashing (2.7) is deferred with the whole auth slice (2.6–2.9) to S3, not removed from scope; hashing stays mandatory when keys land. No control-plane change, so no OpenAPI/client regeneration.

### Task plan
| Id | Task | Pts | State | Evidence |
|----|------|-----|-------|----------|
| 2.1 | Add `chat_stream` to the Provider trait returning a `BoxStream` of `ChatChunk` | 3 | done | `chat::tests::converts_openai_stream_chunks_to_normalized_chunks`; `openai::tests::streams_chunks_until_done` |
| 2.2 | SSE parser for upstream responses (handles partial frames across chunk boundaries) | 5 | done | `sse::tests` (4 tests) cover split/multi-event/CRLF/trailing frames; `maps_malformed_stream_to_error` |
| 2.3 | Re-emit chunks as OpenAI SSE, ending with `[DONE]` | 3 | done | `streams_completion_as_openai_sse`; SDK streaming smoke passed |
| 2.4 | Cancel upstream when client disconnects | 3 | done | `client_disconnect_cancels_upstream_stream`; `cancels_upstream_when_stream_is_dropped` |
| 2.5 | Mid-stream error handling (final SSE error event) | 2 | done | `emits_error_event_without_done_on_cut_stream`; `emits_error_event_on_malformed_stream` |
| 2.6 | SQLite + sqlx setup, migrations: `projects`, `api_keys` | 3 | deferred | Sprint note allows 2.6–2.9 to spill to S3 |
| 2.7 | Key generation and hashing (`gw-<random>`, SHA-256, display prefix) | 2 | deferred | Deferred with the auth slice to S3; hashing remains mandatory when keys land |
| 2.8 | Auth middleware (tower layer) attaching `AuthContext` | 3 | deferred | Deferred with the auth slice to S3 |
| 2.9 | CLI: `cargo run -p server -- key create/revoke/list --project` | 2 | deferred | Deferred with the auth slice to S3 |

### Detailed task cards

#### 2.1 — Provider streaming trait and normalized chunk types
- Acceptance: `chat_stream` returns `BoxStream<'static, Result<ChatChunk, ProviderError>>`; compiles for the OpenAI adapter.
- Depends on: S1 (1.5, 1.7). Files: `crates/core/src/chat.rs`, `crates/providers/src/lib.rs`, `crates/providers/src/openai.rs`, `tests/fixtures/providers/openai/chat-chunks.json`.
- Tests: serde round-trip of `ChatChunk` and `OpenAiStreamChunk`; adapter happy-path streaming against the mock.
- Docs: api-spec.md §6 already defines the SSE contract; no change needed.

#### 2.2 — Upstream SSE parser
- Acceptance: parses frames split across byte-chunk boundaries, multiple events per chunk, CRLF, comments, and `[DONE]`; malformed JSON becomes a stream error.
- Depends on: 2.1. Files: `crates/providers/src/sse.rs`, `tests/fixtures/sse/stream-ok.sse`, `tests/fixtures/sse/stream-malformed.sse`.
- Tests: decoder unit tests for fragmented/multi-event/CRLF input; adapter test maps malformed JSON to `ProviderError::Stream`.
- Docs: test-plan.md §3 fixture list already names `tests/fixtures/sse/`; no change needed.

#### 2.3 — OpenAI SSE re-emission
- Acceptance: chunks are re-emitted lazily as OpenAI `chat.completion.chunk` SSE events with the request id and alias, terminated by `[DONE]`; no full buffering.
- Depends on: 2.1, 2.2. Files: `crates/core/src/chat.rs`, `crates/gateway/src/routes/chat.rs`.
- Tests: integration test reads the body incrementally, asserts chunk framing, model/id, and `[DONE]`; S1 non-streaming tests stay green.
- Docs: api-spec.md §6 unchanged; provider-compatibility.md streaming row already marked supported.

#### 2.4 — Client-disconnect cancellation
- Acceptance: dropping the client response stream drops the provider stream and cancels the upstream request.
- Depends on: 2.3. Files: `crates/providers/src/mock.rs`, `crates/gateway/tests/chat.rs`.
- Tests: mock records early body drops; client aborts after first chunk; assert cancellation observed within a short delay.
- Docs: architecture.md §4.4 already describes drop-based cancellation; no change needed.

#### 2.5 — Mid-stream error handling
- Acceptance: truncated (`stream_cut`) or malformed (`stream_malformed`) upstream streams produce a final `data:` event with an OpenAI-shaped error object and no `[DONE]`.
- Depends on: 2.2, 2.3. Files: `crates/providers/src/openai.rs`, `crates/gateway/src/routes/chat.rs`, `crates/gateway/src/error.rs`.
- Tests: adapter tests for cut/malformed streams; integration assertions on the final error event and absent `[DONE]`.
- Docs: api-spec.md §6/§7 already specify this behavior; no change needed.

### Verification
| Check | Command | Result |
|-------|---------|--------|
| Format | `cargo fmt --all -- --check` | pass |
| Lint | `cargo clippy --offline --all-targets -- -D warnings` | pass |
| Rust tests | `cargo test --offline` | pass — 34 tests (9 integration, 8 core, 12 provider, 5 config) |
| Build | `cargo build --offline` | pass |
| Frontend | `npm run typecheck && npm run lint && npm test` | not applicable — no UI changes |
| OpenAPI drift | `git diff --exit-code -- docs/openapi.json admin-ui/src/api/schema.d.ts` | pass — exit 0, generated client unchanged |
| Migration | `sqlx migrate run` | not applicable — 2.6 deferred |
| E2E | `npm run e2e` | not applicable — S6 onward |
| SDK contract | OpenAI Python SDK 3.24.0 (`OpenAI` client) streaming smoke against local gateway | pass — `SDK_STREAM_PASS`, content `Hello` |
| Security | `cargo audit`, `cargo deny`, secret scan | `cargo audit`/`deny` not applicable — S17/release gate; secret-pattern scan pass — `NO_SECRET_PATTERNS` |
| Baseline (pre-change) | `cargo fmt --all -- --check && cargo clippy --offline --all-targets -- -D warnings && cargo test --offline` | pass — 20 tests |

### Decisions and deviations
- Sprints run directly on `main` (MASTER_PROMPT.md override); no feature branch.
- Committed exactly 16 pts: 2.1–2.5. Auth/DB/CLI (2.6–2.9, 10 pts) deferred to S3 per the S2 sprint note.
- 2.7 (key hashing) is deferred with the whole auth slice, not removed from scope; hashing remains mandatory when keys land (Never cut).
- S1's 1.4 (CI) and 1.10 (UI scaffold) remain deferred; neither is in S2's task table.
- Offline cargo mode used (`--offline`); all newly required crates (`futures-*`, `tokio-util`) are already in the local cache.
- The in-process mock auto-selects `stream_ok` when a request has `stream: true` and no explicit scenario; this keeps SDK streaming usable without a scenario header.
- No control-plane change, so `docs/openapi.json` and `admin-ui/src/api/schema.d.ts` are not regenerated.

### Deferred / carryover
- 2.6 — SQLite + sqlx setup and migrations; sprint note allows 2.6–2.9 to spill to S3.
- 2.7 — Key generation/hashing; deferred with the auth slice to S3, not removed from scope.
- 2.8 — Auth middleware; deferred with the auth slice to S3.
- 2.9 — Key CLI; deferred with the auth slice to S3.
- 1.4 — CI pipeline; carried from S1 (not in S2's table).
- 1.9 — Full upstream error matrix; carried from S1 (minimal mapping remains).
- 1.10 — UI scaffold; carried from S1 (stretch).

### Blockers
- None.

### Definition of done
- [x] Acceptance criteria met for 2.1–2.5
- [x] fmt, clippy, tests pass
- [x] Mock-provider streaming scenarios covered (ok, cut, malformed, client cancel)
- [x] Docs and CHANGELOG updated
- [x] No secrets in logs, fixtures, or repo
- [x] Generated client unchanged (no OpenAPI change)
- [x] Committed on main and pushed

## Sprint S1 — Foundations (MVP)
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
| S1 | MVP | complete | 0d38952 | Foundations: data plane vertical slice |
| S2 | MVP | complete | 07e7419 | Streaming correctness; auth slice deferred to S3 |
