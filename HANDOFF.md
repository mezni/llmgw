# Session handoff

- Date: 2026-11-20
- Sprint / version: S1–S3 deferred sweep / Unreleased
- Branch: main (sprints run directly on main per MASTER_PROMPT.md)
- Commits: `6fb5561`, `b67ed42`, `0452762`, `2e1bc45`, `c813b73`, `a425a2e`, and the final closeout commit at `HEAD`
- Push status: pushed to origin/main
- State: complete

## Completed this session
- Cleared all 28 points of deferred S1–S3 work with no cuts: 1.4, 1.9, 1.10, 2.6–2.9, 3.5, 3.7, and 3.8.
- Added a CI workflow covering Rust fmt/clippy/test/build, sqlx migration, schema checks, frontend typecheck/lint/test/build, OpenAPI drift, and gitleaks.
- Normalized the upstream error matrix for 400/401/403/408/413/429/5xx/network/timeout/stream with sanitized bodies and `Retry-After` handling.
- Added SQLite virtual keys: projects, hashed API keys with display prefixes, active-key lookup, revocation, expiry filtering, auth middleware, and `key create/list/revoke --project` CLI commands.
- Added data-plane auth via `Authorization: Bearer ...`, documented 401 bodies, and usage rows keyed by real project/key IDs.
- Added graceful shutdown that closes the usage channel and flushes the batch writer on SIGTERM/Ctrl-C.
- Added Anthropic SSE streaming translation: `message_start`, `content_block_*`, `message_delta`, `message_stop`, fragmented frames, cut streams, sanitized errors, and gateway SSE smoke coverage.
- Added OpenAI ↔ Anthropic tool-call mapping for definitions, assistant tool calls, tool results, and streamed argument fragments.
- Added a Vite + React + TypeScript admin UI scaffold with `react-router-dom`, local CSS primitives, local mock data, accessible layout, dashboard/projects/keys pages, and Vitest coverage.
- Updated README, CHANGELOG, STATUS, HANDOFF, admin UI docs, provider compatibility, config/operations docs, test plan, and provider docs in the same deferred sweep.

## Test evidence
- `cargo fmt --all -- --check`: pass
- `cargo clippy --offline --all-targets -- -D warnings`: pass
- `cargo test --offline`: pass — 79 tests
- `cargo build --offline`: pass
- Frontend `npm ci --offline`, `npm run typecheck`, `npm run lint`, `npm test`, `npm run build`: pass — 3 tests
- Vite dev server smoke: pass — HTTP 200 and expected page title
- `git diff --exit-code -- docs/openapi.json admin-ui/src/api/schema.d.ts`: pass (exit 0)
- Fresh `sqlx migrate run --source crates/storage/migrations`: pass — `projects`, `api_keys`, `requests`, indexes verified; WAL asserted by the storage test
- OpenAI Python SDK 3.24.0 non-streaming + streaming smoke against a local gateway with a generated key: pass (`echo: Hello` / `Hello`)
- Secret-pattern scan: pass — no matches outside fixtures/mock/docs

## Files changed
- `.github/workflows/ci.yml`, `CHANGELOG.md`, `HANDOFF.md`, `README.md`, `STATUS.md`
- `docs/admin-ui.md`, `docs/config-reference.md`, `docs/operations.md`, `docs/provider-compatibility.md`, `docs/test-plan.md`
- `crates/providers/src/error.rs`, `crates/providers/src/openai.rs`, `crates/providers/src/anthropic.rs`, `crates/providers/src/mock.rs`, provider fixtures
- `crates/gateway/src/error.rs`, `crates/gateway/src/auth.rs`, `crates/gateway/src/router.rs`, `crates/gateway/src/routes/chat.rs`, `crates/gateway/src/usage.rs`, `crates/gateway/tests/chat.rs`
- `crates/storage/src/keys.rs`, `crates/storage/src/keys/repository.rs`, `crates/storage/src/lib.rs`, `crates/storage/src/usage.rs`
- `crates/server/src/main.rs`, `crates/server/tests/shutdown.rs`, `Cargo.lock`
- `admin-ui/**` (Vite/React/TS scaffold, lockfile, mock API, tests, CSS primitives)

## Deferred / carryover
- None from S1–S3 after this pass.
- Future sprint work (S4–S17) remains out of scope.

## Open decisions and risks
- Model-name routing (`claude*` → Anthropic when `GATEWAY_ANTHROPIC_API_KEY` is set) is still temporary until aliases land in S4.
- No control-plane change was made; `docs/openapi.json` and `admin-ui/src/api/schema.d.ts` were intentionally left unchanged.
- The admin UI scaffold uses local mock data only; real admin endpoints, generated OpenAPI client, TanStack Query/Router, shadcn/ui, and Playwright flows remain S6 work.
- `cost_usd` and `attempts` remain null until pricing and retry layers land.

## Gotchas for the next session
- `cargo run -p server` starts the in-process mock when `GATEWAY_UPSTREAM_BASE_URL` is unset; data-plane requests now require a virtual key from `cargo run -p server -- key create --project NAME`.
- The mock scenario selector is `x-mock-scenario` (header or JSON body field); the mock serves both `/v1/chat/completions` and `/v1/messages`.
- Anthropic streaming and tool scenarios use `anthropic_stream_ok`, `anthropic_stream_cut`, `anthropic_stream_tool`, and `tool_use`.
- The core crate directory is `crates/core/` but the package is `gateway-core`.
- `Cargo.lock` and `admin-ui/package-lock.json` are committed; use `--offline` when the network is unavailable.
- The frontend lockfile pins `rolldown` to `1.2.16` and `jsdom` to `16.7.0` so `npm ci --offline` works from the local npm cache.
- Migration filenames must use integer prefixes (`0001_init.sql`); `V0001__init.sql` is rejected by sqlx.

## Next step
Paste MASTER_PROMPT.md with: Sprint = S4
