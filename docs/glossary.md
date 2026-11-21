# Glossary

Terms used across the docs and code. Use these names in identifiers, API fields, UI labels, and commit messages; avoid synonyms like "tenant", "token" (for a virtual key), or "upstream name" (for an alias).

| Term | Meaning |
|------|---------|
| **Gateway** | The Rust service that sits between clients and LLM providers. |
| **Data plane** | Public listener on `:8080` serving `/v1/*` with virtual-key auth. |
| **Control plane / admin plane** | Private listener on `:8081` serving `/admin/v1/*` and the built React UI with cookie auth. |
| **Plane separation** | Two listeners and two auth stacks in one binary; never expose the admin plane publicly (ADR 0003). |
| **Provider** | A configured upstream such as `anthropic` or `openai`; holds encrypted credentials and a base URL. |
| **Provider kind** | Adapter type (`openai`, `anthropic`, `openai_compatible`) that decides wire-format behavior. |
| **Model** | A provider's real upstream model name plus pricing; never exposed directly to clients. |
| **Alias** | Stable client-facing model name (`fast`, `smart`) resolved to an ordered target chain (ADR 0004). |
| **Target** | One model in an alias chain; `position 0` is primary, higher positions are fallbacks. |
| **Route** | The resolution of an alias to its ordered targets, timeout, and attempt limit. V1 has no separate route entity; "route" always means alias resolution. |
| **Fallback** | Trying the next target after a retryable failure, only before the first response byte. |
| **Retry** | Re-attempting the same target before any byte is sent to the client. |
| **Circuit breaker** | Per-provider short-circuit after repeated failures; state visible in 0.3.0+ metrics. |
| **Project** | A billing and isolation boundary owning keys, budgets, and usage rows. Not a tenant or organization. |
| **Virtual key** | Client secret of the form `gw-<base64url>`, hashed at rest, revocable and expirable. Not a provider key. |
| **Key prefix** | The non-secret display fragment (`gw-ab12…`) stored for admin listing. |
| **Allowlist** | Per-key set of aliases it may use; empty means all aliases. |
| **Request id** | `X-Request-Id` value or generated UUID; carried in logs, traces, and request rows. |
| **Tag** | `X-Gateway-Tag` value used only for usage filtering and reporting; never affects routing or billing. |
| **Usage log** | Immutable `requests` row with tokens, cost, latency, status, and attempts. |
| **Attempts** | Per-target outcomes recorded on a request row, including retries and fallbacks. |
| **Spend counter** | Authoritative monthly counter on `projects`, updated on the request path separately from the droppable log channel. |
| **Budget** | Optional monthly USD cap on a project; exceeding it yields 429 `budget_exceeded`. |
| **Config snapshot** | Immutable in-memory view of providers, aliases, pricing, and key cache, swapped atomically on admin mutations. |
| **Static config** | TOML/env values requiring a restart (see [config-reference.md](config-reference.md)). |
| **Dynamic config** | DB-backed values changed through the admin API without a restart. |
| **Master key** | `GATEWAY_MASTER_KEY`, the 32-byte key that encrypts provider credentials at rest. Never stored in the DB. |
| **Log mode** | `off`, `metadata` (default), or `full`; only `full` stores prompt/response bodies. |
| **TTFT** | Time to first byte of a streamed response. |
| **OpenAPI spec** | Generated machine-readable control-plane contract in [openapi.json](openapi.json); feeds the React client. |
| **Data-plane spec** | Human-readable client contract in [api-spec.md](api-spec.md). |
| **Fixture** | Scrubbed recorded payload used by adapter and contract tests. |
| **Mock provider** | Local `axum` server with deterministic scenarios used by integration and load tests. |
| **Exact cache / semantic cache** | V2 exact-match and V3 embedding-based response caches; both opt-in and project-scoped by default. |
| **Guardrail** | V2 rule that redacts or blocks content; decisions are logged, matched content is not. |
