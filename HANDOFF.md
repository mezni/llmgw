# Session handoff

- Date: 2026-11-20
- Sprint / version: S1 / MVP (targeting 0.1.0)
- Branch: main (sprints run directly on main per MASTER_PROMPT.md)
- HEAD commit: 83f45cf (sprint closeout)
- Push status: pushed to origin/main
- State: complete

## Completed this session
- Created the Cargo workspace with `core` (packaged as `gateway-core`), `providers`, `storage`, `gateway`, and `server`.
- Added normalized chat types, validation, scrubbed OpenAI fixtures, provider trait, OpenAI adapter with error classification, and an axum mock provider with `ok`/`delay`/`429`/`500`/`400` scenarios.
- Added the axum data plane: `GET /health`, `POST /v1/chat/completions`, request IDs on responses and JSON log lines, OpenAI-shaped errors, and TOML + env config with precedence and validation.
- Wired the server to an in-process mock by default or `GATEWAY_UPSTREAM_BASE_URL`/`GATEWAY_UPSTREAM_API_KEY` for a real upstream.
- Updated README, config-reference.md, CHANGELOG.md, and STATUS.md.

## Test evidence
- `cargo fmt --all -- --check`: pass
- `cargo clippy --offline --all-targets -- -D warnings`: pass
- `cargo test --offline`: pass — 20 tests (6 core, 4 adapter, 5 integration, 5 config)
- `cargo build --offline`: pass
- OpenAI Python SDK 3.24.0 smoke against local gateway: pass (`SDK_SMOKE_PASS`)
- Frontend/E2E/OpenAPI/migration/security checks: not applicable to S1 scope

## Files changed
- `Cargo.toml`, `Cargo.lock`, `.gitignore`, `config.example.toml`
- `crates/core/**`, `crates/providers/**`, `crates/gateway/**`, `crates/server/**`, `crates/storage/**`
- `tests/fixtures/providers/openai/**`
- `README.md`, `CHANGELOG.md`, `docs/config-reference.md`, `STATUS.md`, `MASTER_PROMPT.md` (committed earlier on main)

## Deferred / carryover
- 1.4 — CI pipeline; S1 note says drop to S2.
- 1.9 — full upstream error matrix; minimal OpenAI-shaped errors are included in 1.8.
- 1.10 — Vite + React + TS admin UI scaffold; stretch item.

## Open decisions and risks
- S1 commits 18 pts against a ~16 pt target because 1.6 and 1.8 are both required for the demo.
- `stream: true` is rejected until S2; document prominently when demoing.
- Unknown TOML keys are ignored in S1 although config-reference.md specifies a warning; align in a later sprint.
- No virtual-key auth yet: do not expose the listener publicly.

## Gotchas for the next session
- `cargo run -p server` uses the in-process mock when `GATEWAY_UPSTREAM_BASE_URL` is unset, so it works offline.
- The scenario selector is `x-mock-scenario` (header or JSON body field).
- The core crate directory is `crates/core/` but the package is `gateway-core` to avoid the std `core` name collision.
- `Cargo.lock` is committed; use `--offline` when the network is unavailable.

## Next step
Paste MASTER_PROMPT.md with: Sprint = S2
