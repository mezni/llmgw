# Test plan

How the gateway is tested, which scenarios the mock provider owns, what fixtures live in the repo, and what runs automatically. The short strategy table is in [architecture §13](architecture.md#13-testing-strategy); this is the operational reference.

## 1. Test levels

| Level | Scope | Tooling | Gate |
|-------|-------|---------|------|
| Unit | Core types, cost calculation, backoff, token buckets, alias resolution, config merge | `cargo test` | Every push |
| Adapter | Provider request/response mapping and SSE parsing against recorded payloads | `cargo test` + fixtures | Every push |
| Integration | Full gateway against the mock provider on `:8080` | `axum` test server, fresh SQLite DB per test | Every push |
| Admin | CRUD, auth, audit rows, snapshot reload, OpenAPI snapshot | `cargo test` with in-memory SQLite | Every push |
| Contract | Official OpenAI Python and TypeScript SDKs against the gateway | `pytest` / `node:test` | Every PR |
| UI component | Forms, states, generated-client typing | Vitest + React Testing Library + MSW | Every push |
| E2E | Admin UI smoke flows against the built app on `:8081` | Playwright | Every PR (S6 onward) |
| Migration | Fresh `sqlx migrate run`, schema snapshot, backup/restore | `cargo test` + scripted SQLite | Every push |
| Security | Dependency audit, secret scanning, log redaction assertions | `cargo audit`, `cargo deny`, gitleaks-style scan | Nightly + release |
| Load / chaos | Concurrent streams, disconnect storms, mock failures | `oha` or `k6` | Manual before release (S7, S17) |
| Real provider smoke | Small prompts against configured providers | Script with test-only keys | Manual, never required for CI |

No test uses a real provider key or a real prompt. The only exception is the deliberate manual smoke, which uses a dedicated low-budget credential.

## 2. Mock provider scenarios

The mock is an `axum` server built in M1 and selected by a header or config. Every scenario is reusable by unit, integration, and load tests.

| Scenario | Behavior | What it proves |
|----------|----------|----------------|
| `ok` | Valid non-streaming JSON response | Passthrough, alias/model echo, usage logging |
| `ok_with_tools` | Response with `tool_calls` | Tool round-trip and storage |
| `slow` | Delays first byte beyond the alias timeout | Timeout, 504, in-flight snapshot behavior |
| `rate_limited` | 429 with `Retry-After: 1` | Retry/backoff and normalized 429 |
| `server_error` | 500 with a hostile body containing a fake key | Error sanitization, no credential echo |
| `bad_request` | 400 with an OpenAI-shaped error | Upstream 400 mapping, no retry |
| `flaky_then_ok` | Fails once, succeeds on second attempt | Retry counting and `attempts_json` |
| `all_targets_fail` | Every target returns 502 | Fallback exhaustion and normalized 502 |
| `stream_ok` | Chunked SSE, including usage-only final chunk | SSE framing, TTFT, usage capture |
| `stream_tool_fragments` | Tool-call deltas split across chunks | Index-based assembly |
| `stream_cut` | Closes the TCP stream mid-response | Final error event without `[DONE]`, cancellation cleanup |
| `stream_malformed` | Invalid JSON lines and no `[DONE]` | Malformed SSE detection and error mapping |
| `client_cancel` | Client drops the response body | Upstream cancellation, `client_cancelled` log status |
| `huge_body` | Returns a body over the configured limit | 413 and no partial log write |
| `usage_missing` | Omits `usage` | Gateway-side accounting still works |

Each scenario records the upstream request it received so tests can assert header propagation, key isolation, and that no client virtual key is forwarded upstream.

## 3. Fixtures

- `tests/fixtures/providers/openai/` — non-streaming, streamed, tool, error, and usage payloads from real OpenAI-compatible responses with all identifiers scrubbed.
- `tests/fixtures/providers/anthropic/` — `message_start`/`content_block_*`/`message_delta` sequences, tool blocks, cache-token usage, and error bodies.
- `tests/fixtures/sdk/` — golden responses the Python and TypeScript SDKs must accept, including streaming errors.
- `tests/fixtures/admin/` — representative OpenAPI responses for each resource; also used by MSW in UI component tests.
- `tests/fixtures/db/` — schema snapshot from a fresh migration run and a small seeded dataset used by dashboard queries.
- `tests/fixtures/sse/` — raw chunk sequences, including malformed and cut streams, byte-for-byte.

Fixtures never contain real secrets, real prompt content, or real customer identifiers. Fixture drift is a test failure: an adapter change must update its fixture and this plan in the same commit (see [provider-compatibility.md](provider-compatibility.md)).

## 4. What runs in CI

**On every push:**

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test` (unit, adapter, integration, admin, migration)
- `npm run typecheck && npm run lint && npm test`
- OpenAPI and generated-client drift check from [admin-api.md §5](admin-api.md#5-openapi-spec-and-generated-client)

**On every pull request:**

- Everything above, plus the OpenAI SDK contract suite and the Playwright smoke flows against the built app.

**Nightly:**

- `cargo audit`, `cargo deny`, secret scan, and a fresh backup/restore rehearsal.

**Manual before a release:**

- Load/chaos suite, real-provider smoke, and the upgrade rehearsal from the previous version (see [operations.md](operations.md#6-upgrading)).

CI never needs Internet access to a provider. A license or API outage must not block a merge.

## 5. Definition of done for a test change

A feature is not done until its tests cover the happy path, one validation failure, one provider failure, and the log/audit side effect. Streaming features additionally require a cut-stream and a malformed-SSE case. See [conventions.md](conventions.md) for the general definition of done.
