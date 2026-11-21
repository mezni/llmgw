# Session handoff

- Date: 2026-11-20
- Sprint / version: S3 / MVP (0.1.0 checkpoint)
- Branch: main (sprints run directly on main per MASTER_PROMPT.md)
- S3 commits: 2998432, 8d18abc, and the S3 closeout commit at `HEAD`
- Push status: pushed to origin/main
- State: complete

## Completed this session
- Added the `requests` SQLite schema with project/key/alias/time/status indexes and an integer-prefixed `0001_init.sql` migration.
- Added `UsageEvent`, bounded `UsageLog::try_send` logging, and a background `UsageWriter` that flushes by batch size or interval.
- Tapped provider streams for TTFT and final usage; dropped streams emit `client_cancelled`, upstream stream errors emit `error`.
- Added `ProviderError::kind()` and shared `parse_retry_after`; OpenAI adapter now uses the shared parser.
- Added a non-streaming Anthropic Messages adapter: system extraction, `max_tokens` default 1024, role validation, text/cache usage mapping, and sanitized errors.
- Added temporary model-name provider selection (`claude*` → Anthropic when `GATEWAY_ANTHROPIC_API_KEY` is set; other models → OpenAI-compatible provider).
- Updated config, provider compatibility, data model, test plan, observability, README, ADR 0004, STATUS.md, CHANGELOG.md, and this handoff.

## Test evidence
- `cargo fmt --all -- --check`: pass
- `cargo clippy --offline --all-targets -- -D warnings`: pass
- `cargo test --offline`: pass — 52 tests (12 gateway integration, 6 gateway unit, 10 core, 16 provider, 5 config, 3 storage)
- `cargo build --offline`: pass
- Fresh `sqlx migrate run --source crates/storage/migrations` against `target/migration-test-s3.db`: `1/migrate init`; `requests` columns and 4 indexes verified
- `git diff --exit-code -- docs/openapi.json admin-ui/src/api/schema.d.ts`: pass (exit 0)
- OpenAI Python SDK 3.24.0 non-streaming and streaming smoke against local gateway: pass (`echo: Hello` / `Hello`)
- Secret-pattern scan: pass (no matches outside fixtures/mock/docs)

## Files changed
- `Cargo.lock`, `README.md`, `STATUS.md`, `CHANGELOG.md`, `HANDOFF.md`
- `docs/adr/0004-aliases-over-model-names.md`, `docs/config-reference.md`, `docs/data-model.md`, `docs/observability.md`, `docs/provider-compatibility.md`, `docs/test-plan.md`
- `crates/core/src/usage.rs`, `crates/core/src/lib.rs`
- `crates/storage/Cargo.toml`, `crates/storage/migrations/0001_init.sql`, `crates/storage/src/lib.rs`, `crates/storage/src/usage.rs`
- `crates/providers/Cargo.toml`, `crates/providers/src/anthropic.rs`, `crates/providers/src/error.rs`, `crates/providers/src/lib.rs`, `crates/providers/src/mock.rs`, `crates/providers/src/openai.rs`, `crates/providers/tests/fixtures/providers/anthropic/message-response.json`
- `crates/gateway/Cargo.toml`, `crates/gateway/src/lib.rs`, `crates/gateway/src/provider_registry.rs`, `crates/gateway/src/router.rs`, `crates/gateway/src/routes/chat.rs`, `crates/gateway/src/usage.rs`, `crates/gateway/tests/chat.rs`
- `crates/server/Cargo.toml`, `crates/server/src/config.rs`, `crates/server/src/main.rs`

## Deferred / carryover
- 3.7 — Anthropic streaming event translation; Anthropic streaming returns `400 unsupported_field` until this lands.
- 3.8 — Tool-call mapping between formats; Anthropic tools return `400 unsupported_field` until this lands.
- 3.5 — Graceful shutdown that drains requests and the log channel.
- 2.6–2.9 — S2 auth spill (SQLite keys, hashing, middleware, CLI); user-approved deferral, key hashing remains mandatory when the slice lands.
- 1.4 — CI pipeline; carried from S1.
- 1.9 — Full upstream error matrix; carried from S1.
- 1.10 — UI scaffold; carried from S1.

## Open decisions and risks
- Committed exactly 18 pts (3.1–3.4, 3.6, 3.9) with user approval.
- Key hashing (2.7) is deferred a second time, a documented deviation from the "Never cut" rule.
- With no virtual keys yet, usage rows use local placeholders `local` / `local-key`; do not expose the listener publicly.
- Model selection is temporary and will be replaced by aliases in S4; see ADR 0004.
- `cost_usd` and `attempts` remain null until pricing and retry layers land.
- No control-plane change, so `docs/openapi.json` and `admin-ui/src/api/schema.d.ts` were intentionally left unchanged.

## Gotchas for the next session
- `cargo run -p server` uses the in-process mock when `GATEWAY_UPSTREAM_BASE_URL` is unset; it now also creates/migrates `DATABASE_URL` and starts the usage writer.
- The scenario selector is `x-mock-scenario` (header or JSON body field); the mock now serves both `/v1/chat/completions` and `/v1/messages`.
- The core crate directory is `crates/core/` but the package is `gateway-core`.
- `Cargo.lock` is committed; use `--offline` when the network is unavailable.
- Migration filenames must use integer prefixes (`0001_init.sql`); `V0001__init.sql` is rejected by sqlx.

## Next step
Paste MASTER_PROMPT.md with: Sprint = S4

## S2 history
- S2 committed exactly 16 pts (2.1–2.5): streaming trait, SSE parser, OpenAI streaming adapter, OpenAI SSE re-emission, disconnect cancellation, and mid-stream errors.
- S2 test evidence: fmt/lint/tests/build/OpenAPI/SDK streaming smoke/secret scan all passed; auth/DB/CLI spilled to S3.
