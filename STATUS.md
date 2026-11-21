# Status

## Current work: S1–S3 deferred sweep (MVP backlog)
- Branch: main (sprints run directly on main per MASTER_PROMPT.md)
- Started / updated: 2026-11-20
- State: in progress
- Scope: all 28 pts of deferred S1–S3 work, approved by the user; no further deferral from this set.

This is a backlog-clearing pass, not a new numbered sprint and not a version checkpoint. It closes tasks carried from S1 (1.4, 1.9, 1.10), S2 (2.6–2.9), and S3 (3.5, 3.7, 3.8). The 0.1.0 release already exists in CHANGELOG.md; new work goes under `[Unreleased]`. No control-plane API changes are in scope, so `docs/openapi.json` and `admin-ui/src/api/schema.d.ts` remain unchanged.

### Task plan
| Id | Task | Pts | State | Evidence |
|----|------|-----|-------|----------|
| 1.4 | CI: fmt, clippy, test on push | 2 | planned | workflow file; local reproduction of every CI step |
| 1.9 | Full upstream error matrix | 2 | planned | provider error tests cover 400/401/403/408/413/429/5xx/network/timeout/stream |
| 1.10 | Vite + React + TS admin UI scaffold | 3 | planned | dev server runs; typecheck/lint/unit tests pass; mock API only |
| 2.6 | SQLite + sqlx setup, migrations: `projects`, `api_keys` | 3 | planned | migration test asserts both tables, WAL-enabled connection, indexes |
| 2.7 | Key generation and hashing (`gw-<random>`, SHA-256, display prefix) | 2 | planned | secret shown once; DB stores hash/prefix only |
| 2.8 | Auth middleware attaching `AuthContext` | 3 | planned | missing/invalid/revoked/expired keys return documented 401 body |
| 2.9 | CLI: `key create/revoke/list --project` | 2 | planned | CLI tests or scripted smoke against a temp DB |
| 3.5 | Graceful shutdown draining requests and log channel | 3 | planned | SIGTERM test shows no lost usage events |
| 3.7 | Anthropic streaming event translation | 5 | planned | fragmented SSE fixture maps events to `ChatChunk`; gateway SSE smoke |
| 3.8 | Tool-call mapping between formats | 3 | done | OpenAI/Anthropic tool round-trip tests and streamed tool-fragment tests pass |

### Detailed task cards

#### 1.4 — CI pipeline
- Acceptance (verbatim): "Pipeline fails on warnings."
- Files: `.github/workflows/ci.yml`, `docs/test-plan.md`.
- Tests: workflow runs fmt, clippy `-D warnings`, Rust tests/build, migration, OpenAPI drift, frontend typecheck/lint/unit, and secret scan; every command is also reproduced locally.

#### 1.9 — Full upstream error matrix
- Acceptance (verbatim): "Upstream 400/429/500 produce correct status and error body."
- Files: `crates/providers/src/error.rs`, `crates/providers/src/openai.rs`, `crates/providers/src/anthropic.rs`, `crates/gateway/src/error.rs`, fixtures/tests.
- Tests: status/code/type mapping for auth, bad request, rate limit with `Retry-After`, timeout, 5xx, network, malformed stream, and sanitized hostile bodies; no credential echo.

#### 1.10 — Vite + React + TS admin UI scaffold
- Acceptance (verbatim): "Dev server runs; typecheck/lint/unit tests pass; no real credentials or admin endpoints used."
- Files: `admin-ui/**`, `docs/admin-ui.md`, `docs/test-plan.md`.
- Tests: `npm run typecheck && npm run lint && npm test`; dev server smoke; mock API layer has no real URLs or secrets.

#### 2.6 — SQLite + sqlx setup
- Acceptance (verbatim): "`sqlx migrate run` works; WAL mode enabled."
- Files: `crates/storage/migrations/0001_init.sql`, `crates/storage/src/lib.rs`, `docs/data-model.md`.
- Tests: migration test asserts `projects`, `api_keys`, indexes, and WAL pragmas.

#### 2.7 — Key generation and hashing
- Acceptance (verbatim): "Secret shown once; only the hash is stored."
- Files: `crates/storage/src/keys.rs`, `crates/storage/src/lib.rs`, `crates/storage/src/keys/repository.rs`, docs.
- Tests: generated secret shape `gw-<base64url>`, display prefix, SHA-256 hash, no secret in DB row, duplicate hash rejected.

#### 2.8 — Auth middleware
- Acceptance (verbatim): "Missing, invalid, and revoked keys return 401 with the correct error body."
- Files: `crates/gateway/src/auth.rs`, `crates/gateway/src/router.rs`, `crates/gateway/tests/chat.rs`, `docs/api-spec.md`.
- Tests: valid key passes and attaches `AuthContext`; missing/invalid/revoked/expired keys get `401 authentication_error/invalid_api_key`; usage rows use real project/key ids.

#### 2.9 — Key CLI
- Acceptance (verbatim): "Usable from the terminal against the DB."
- Files: `crates/server/src/main.rs`, `docs/config-reference.md`, `docs/operations.md`.
- Tests: `key create` prints the secret once, `key list` shows prefix only, `key revoke` makes auth fail.

#### 3.5 — Graceful shutdown
- Acceptance (verbatim): "No lost events on SIGTERM in a test."
- Files: `crates/server/src/main.rs`, `crates/storage/src/usage.rs`, `crates/gateway/src/usage.rs`.
- Tests: shutdown closes the log channel, flushes the batch writer, and test observes all rows.

#### 3.7 — Anthropic streaming translation
- Acceptance (verbatim): "Streaming works through the same OpenAI-format endpoint."
- Files: `crates/providers/src/anthropic.rs`, `crates/providers/src/mock.rs`, fixtures, `docs/provider-compatibility.md`.
- Tests: `message_start`/`content_block_*`/`message_delta`/`message_stop` translated to `ChatChunk`, fragmented frames, errors, and gateway SSE smoke.

#### 3.8 — Tool-call mapping
- Acceptance (verbatim): "A tool-calling round trip works on both providers."
- Files: `crates/providers/src/openai.rs`, `crates/providers/src/anthropic.rs`, `crates/providers/src/mock.rs`, fixtures, docs.
- Tests: OpenAI tool definitions/results round trip; Anthropic `tools`/`tool_use`/`tool_result` round trip; streamed tool fragments assemble.

### Verification
| Check | Command | Result |
|-------|---------|--------|
| Format | `cargo fmt --all -- --check` | pending |
| Lint | `cargo clippy --offline --all-targets -- -D warnings` | pending |
| Rust tests | `cargo test --offline` | pending |
| Build | `cargo build --offline` | pending |
| Frontend | `npm run typecheck && npm run lint && npm test` in `admin-ui` | pending |
| OpenAPI drift | `git diff --exit-code -- docs/openapi.json admin-ui/src/api/schema.d.ts` | pending |
| Migration | fresh `sqlx migrate run` + schema/pragmas check | pending |
| E2E | `npm run e2e` in `admin-ui` | pending |
| SDK contract | OpenAI Python SDK non-streaming + streaming smoke | pending |
| Security | secret-pattern scan | pending |

### Decisions and deviations
- All deferred S1–S3 work is in scope by explicit user choice; no deferred item from this set is cut.
- Work is done directly on `main`; no branch or version roll.
- Key hashing (2.7) lands now after two documented deferrals; it remains a Never-cut requirement.
- The UI scaffold uses a local mock API layer and local UI primitives; the generated OpenAPI client and generated shadcn components remain S6 work.
- No control-plane change: OpenAPI and generated TypeScript client stay unchanged.
- SIGTERM draining uses the existing axum graceful shutdown plus explicit usage-channel closure and writer flush.

### Deferred / carryover
- None from S1–S3 after this pass. Future sprint work (S4–S17) remains out of scope.

### Blockers
- None.

### Definition of done
- [ ] Acceptance criteria met for all committed tasks
- [ ] fmt, clippy, tests pass
- [ ] Frontend typecheck, lint, unit tests pass
- [ ] OpenAPI/client drift check passes
- [ ] Migration and SDK contract checks pass
- [ ] No secrets in logs, fixtures, or repo
- [ ] Docs and CHANGELOG updated
- [ ] Committed on main and pushed

## Sprint S3 — Usage logging and second provider (MVP, 0.1.0 checkpoint)
- Branch: main (sprints run directly on main per MASTER_PROMPT.md)
- Started / updated: 2026-11-20
- State: complete

Committed scope: 18 pts (3.1, 3.2, 3.3, 3.4, 3.6, 3.9). The S3 table lists 29 pts and S2 spilled 10 more; 18 is the smallest set that delivers the MVP demo (usage in SQL, TTFT for streams, and two providers behind `/v1/chat/completions`) at approximately the 16-point capacity. Cuts follow sprint-planning.md §5: 3.8 (tool-call mapping) first, then 3.7 (Anthropic streaming translation) and 3.5 (graceful shutdown) for capacity. The S2 auth slice (2.6–2.9), including key hashing, is deferred again with the user's explicit approval of that Never-cut deviation. No control-plane change, so no OpenAPI/client regeneration. This is the 0.1.0 version checkpoint.

### Task plan
| Id | Task | Pts | State | Evidence |
|----|------|-----|-------|----------|
| 3.1 | `requests` table and indexes | 2 | done | `0001_init.sql`; fresh migration + `.schema requests` assertion |
| 3.2 | `UsageEvent` type and bounded mpsc channel with `try_send` | 2 | done | `full_channel_record_does_not_block`; metadata-only event test |
| 3.3 | Background batch writer (N events or T ms per transaction) | 3 | done | `flushes_when_batch_is_full`; `flushes_on_interval_and_on_channel_close` |
| 3.4 | Stream tap: TTFT, final usage, emits on end or drop | 5 | done | usage tap unit tests + `logs_stream_usage_and_ttft`; cancellation test |
| 3.6 | Anthropic adapter: request mapping (system prompt, required `max_tokens`, roles) | 5 | done | 4 adapter tests + `routes_claude_models_to_anthropic`; fixture added |
| 3.9 | Provider selection by model name (temporary, replaced by aliases in S4) | 1 | done | `resolves_claude_models_to_anthropic`; `routes_claude_models_to_anthropic` |

### Detailed task cards

#### 3.1 — `requests` table and indexes
- Acceptance (verbatim): "Migration applied; schema matches the architecture doc."
- Depends on: S1 (1.1), S2 streaming (2.1–2.5). Files: `crates/storage/migrations/0001_init.sql`, `crates/storage/src/**`, `docs/data-model.md`.
- Tests: fresh `sqlx migrate run`; schema assertion for `requests` columns and indexes from data-model.md §2/§3.
- Docs: data-model.md §2/§4 notes the initial migration subset.

#### 3.2 — `UsageEvent` and bounded channel
- Acceptance (verbatim): "Handlers never block on logging."
- Depends on: 3.1. Files: `crates/core/src/usage.rs`, `crates/gateway/src/usage.rs`, crate `lib.rs` exports.
- Tests: `try_send` on a capacity-1 channel returns immediately when full; event round-trips the documented metadata fields with no prompt/response bodies or secrets.
- Docs: security.md §6 and observability.md §1 already define the metadata-only field set.

#### 3.3 — Background batch writer
- Acceptance (verbatim): "Rows appear after a short delay; failures logged, never propagated."
- Depends on: 3.1, 3.2. Files: `crates/storage/src/usage.rs`, `crates/gateway/src/usage.rs`, `crates/server/src/main.rs`.
- Tests: batched insert into a temp SQLite DB after batch-size or interval flush; closed/failed channel and bad-row paths do not panic or fail the request path.
- Docs: architecture.md §5 and data-model.md §1 already describe WAL and best-effort writes.

#### 3.4 — Stream tap
- Acceptance (verbatim): "Cancelled streams are logged with `client_cancelled`."
- Depends on: 3.2, S2 (2.1–2.5). Files: `crates/gateway/src/usage.rs`, `crates/gateway/src/routes/chat.rs`.
- Tests: completed stream emits one row with TTFT and final usage; dropped stream emits one `client_cancelled` row with observed tokens; mid-stream error emits `error` without `[DONE]`.
- Docs: api-spec.md §6 and architecture.md §4.4 already specify this behavior.

#### 3.6 — Anthropic non-streaming adapter
- Acceptance (verbatim): "Non-streaming call works against the real API."
- Depends on: S1 (1.5, 1.7). Files: `crates/providers/src/anthropic.rs`, `crates/providers/src/lib.rs`, `crates/providers/src/mock.rs`, `tests/fixtures/providers/anthropic/**`.
- Tests: request mapping (system extraction, `max_tokens`, roles), response mapping, cache-token usage, sanitized error classification against a local Anthropic-shaped mock; real-API smoke recorded as manual evidence, never in CI.
- Docs: provider-compatibility.md marks non-streaming supported and streaming translation deferred; test-plan.md lists the new fixtures.

#### 3.9 — Provider selection by model name
- Acceptance (verbatim): "Request for a Claude model goes to Anthropic."
- Depends on: 3.6. Files: `crates/gateway/src/router.rs`, `crates/gateway/src/routes/chat.rs`, `crates/server/src/main.rs`.
- Tests: Claude model routes to Anthropic mock; other models route to OpenAI mock; response still echoes the client's model alias.
- Docs: ADR 0004 and api-spec.md §1 note this temporary selector is replaced by aliases in S4.

### Verification
| Check | Command | Result |
|-------|---------|--------|
| Format | `cargo fmt --all -- --check` | pass |
| Lint | `cargo clippy --offline --all-targets -- -D warnings` | pass |
| Rust tests | `cargo test --offline` | pass — 52 tests (12 gateway integration, 6 gateway unit, 10 core, 16 provider, 5 config, 3 storage) |
| Build | `cargo build --offline` | pass |
| Frontend | `npm run typecheck && npm run lint && npm test` | not applicable — no UI changes |
| OpenAPI drift | `git diff --exit-code -- docs/openapi.json admin-ui/src/api/schema.d.ts` | pass (exit 0) |
| Migration | fresh `sqlx migrate run` against a temp DB + schema check | pass — `1/migrate init`; `requests` columns and 4 indexes verified |
| E2E | `npm run e2e` | not applicable — S6 onward |
| SDK contract | OpenAI Python SDK 3.24.0 non-streaming + streaming smoke | pass — `echo: Hello` / `Hello` |
| Security | secret-pattern scan | pass — no matches outside fixtures/mock/docs |

### Decisions and deviations
- Sprints run directly on `main` (MASTER_PROMPT.md override); no feature branch.
- Scope approved by the user: 3.1–3.4, 3.6, 3.9 (18 pts). Key hashing (2.7) is deferred a second time with explicit approval, a documented deviation from the "Never cut" rule; it remains mandatory when the auth slice lands.
- Cut 3.8 first per sprint-planning.md §5; 3.7 and 3.5 cut for capacity, not removed from the backlog.
- With no virtual keys yet, request-log rows use the documented local placeholders `local` / `local-key`; no client secret is stored.
- Model selection is temporary: names beginning with `claude` route to Anthropic when `GATEWAY_ANTHROPIC_API_KEY` is set; ADR 0004 records the S4 alias replacement.
- Anthropic streaming translation (3.7) is deferred, so Anthropic-backed streaming requests return `400 unsupported_field` until 3.7 lands; provider-compatibility.md is updated to match.
- No control-plane change, so `docs/openapi.json` and `admin-ui/src/api/schema.d.ts` remain unchanged.
- 0.1.0 version checkpoint: convert `[Unreleased]` to a dated 0.1.0 release in CHANGELOG.md and start a new `[Unreleased]` section.

### Deferred / carryover
- 3.7 — Anthropic streaming event translation; capacity, after first cut in §5.
- 3.8 — Tool-call mapping between formats; first cut in §5.
- 3.5 — Graceful shutdown that drains requests and the log channel; capacity.
- 2.6–2.9 — S2 auth spill (SQLite keys, hashing, middleware, CLI); user-approved deferral.
- 1.4 — CI pipeline; carried from S1.
- 1.9 — Full upstream error matrix; carried from S1.
- 1.10 — UI scaffold; carried from S1.

### Blockers
- None.

### Definition of done
- [x] Acceptance criteria met for committed tasks
- [x] fmt, clippy, tests pass
- [x] Mock-provider scenarios covered; real-provider evidence recorded where relevant
- [x] Docs and CHANGELOG updated
- [x] No secrets in logs, fixtures, or repo
- [x] Generated client unchanged (no OpenAPI change)
- [x] Committed on main and pushed

## Sprint S2 — Streaming and auth (MVP)
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
| S2 | MVP | complete | 2c4daa8..41b12a8 | Streaming correctness; auth slice deferred to S3 |
| S3 | MVP | complete | 2998432..HEAD | Usage logging, stream tap, Anthropic non-streaming, temporary model routing; 0.1.0 checkpoint |
