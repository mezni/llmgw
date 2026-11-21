# Session handoff

- Date: 2026-11-20
- Sprint / version: S2 / MVP (targeting 0.1.0)
- Branch: main (sprints run directly on main per MASTER_PROMPT.md)
- HEAD commit: 07e7419 (sprint closeout)
- Push status: pushed to origin/main
- State: complete

## Completed this session
- Added normalized streaming types (`ChatChunk`, `Delta`, tool-call deltas, OpenAI wire shapes) and extended the `Provider` trait with `chat_stream`.
- Implemented an upstream SSE decoder that handles fragmented frames, multiple events per chunk, CRLF, comments, and trailing events; malformed JSON becomes a stream error.
- Implemented OpenAI streaming adapter with `stream_options.include_usage`, cancellation on dropped streams, and mid-stream error classification.
- Re-emitted provider chunks as OpenAI-compatible SSE with request id/model, `[DONE]` terminator, and a final error event without `[DONE]` on upstream failure.
- Added mock scenarios `stream_ok`, `stream_cut`, `stream_malformed`, `client_cancel` with cancellation counters; default `ok` + `stream: true` selects `stream_ok`.
- Updated README, CHANGELOG.md, STATUS.md, and this handoff.

## Test evidence
- `cargo fmt --all -- --check`: pass
- `cargo clippy --offline --all-targets -- -D warnings`: pass
- `cargo test --offline`: pass — 34 tests (9 gateway integration, 8 core, 12 provider, 5 config)
- `cargo build --offline`: pass
- `git diff --exit-code -- docs/openapi.json admin-ui/src/api/schema.d.ts`: pass (exit 0)
- OpenAI Python SDK 3.24.0 (`OpenAI` client) streaming smoke against local gateway: pass (`SDK_STREAM_PASS`, content `Hello`)
- Secret-pattern scan: pass (`NO_SECRET_PATTERNS`)
- Frontend/E2E/migration/security audits: not applicable to S2 scope

## Files changed
- `Cargo.lock`, `README.md`, `STATUS.md`, `CHANGELOG.md`, `HANDOFF.md`
- `crates/core/src/chat.rs`, `tests/fixtures/providers/openai/chat-chunks.json`
- `crates/providers/Cargo.toml`, `crates/providers/src/lib.rs`, `crates/providers/src/openai.rs`, `crates/providers/src/sse.rs`, `crates/providers/src/mock.rs`, `tests/fixtures/sse/**`
- `crates/gateway/Cargo.toml`, `crates/gateway/src/routes/chat.rs`, `crates/gateway/tests/chat.rs`

## Deferred / carryover
- 2.6 — SQLite + sqlx setup and migrations (`projects`, `api_keys`).
- 2.7 — Key generation and hashing (`gw-<random>`, SHA-256, display prefix); deferred with the auth slice, not removed (Never cut).
- 2.8 — Auth middleware attaching `AuthContext`.
- 2.9 — Key CLI (`cargo run -p server -- key create/revoke/list --project`).
- 1.4 — CI pipeline; carried from S1.
- 1.9 — Full upstream error matrix; carried from S1.
- 1.10 — UI scaffold; carried from S1.

## Open decisions and risks
- Committed exactly 16 pts (2.1–2.5) per the S2 note; auth/DB/CLI spill to S3.
- No virtual-key auth yet: do not expose the listener publicly.
- Mid-stream failures are not retried; the final SSE error event is the only signal after the first chunk.
- Unknown TOML keys are still ignored although config-reference.md specifies a warning; align in a later sprint.

## Gotchas for the next session
- `cargo run -p server` uses the in-process mock when `GATEWAY_UPSTREAM_BASE_URL` is unset, so streaming works offline.
- The scenario selector is `x-mock-scenario` (header or JSON body field); streaming requests with the default `ok` scenario route to `stream_ok`.
- The core crate directory is `crates/core/` but the package is `gateway-core` to avoid the std `core` name collision.
- `Cargo.lock` is committed; use `--offline` when the network is unavailable.
- No control-plane change in S2, so `docs/openapi.json` and `admin-ui/src/api/schema.d.ts` were intentionally left unchanged.

## Next step
Paste MASTER_PROMPT.md with: Sprint = S3
