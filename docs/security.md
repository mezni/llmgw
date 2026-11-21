# Security

Security model for the gateway. The short control summary lives in [architecture §9](architecture.md#9-security); this document explains the threats behind it and the rules implementations must follow.

## 1. Assets

- **Provider credentials** — upstream API keys for Anthropic, OpenAI, and others. Compromise means direct financial loss.
- **Client virtual keys** — `gw-…` secrets issued to projects. Compromise allows unauthorized LLM spend attributed to a project.
- **Admin credentials and sessions** — control over projects, budgets, keys, aliases, and provider configuration.
- **Request data** — prompts, responses, tool calls, and usage records. Sensitive by default even when logging is metadata-only.
- **Configuration snapshot** — routing, pricing, limits, and provider endpoints; tampering redirects traffic or changes cost accounting.

## 2. Threat model

| Adversary | Capability | Mitigation |
|-----------|------------|------------|
| Client holding a virtual key | Calls the data plane, tries other aliases, floods requests | Per-key alias allowlist, rate limits, budgets, body-size limits, key revocation/expiry |
| Compromised client application | Key leaked from a project repo or `.env` | Secret shown once, stored only as a hash, revocable, per-project scope; no provider key is ever given to clients |
| Network observer on the client link | Reads or modifies traffic | TLS terminated at Caddy; gateway serves HTTP only behind the proxy |
| Compromised admin session | Attempts CRUD on projects, keys, providers | HttpOnly + SameSite=Strict cookie, login throttling, Argon2 password hashing, audit log, scoped roles in V2 |
| Operator with DB or volume access | Reads stored rows | Provider credentials encrypted at rest; master key supplied via environment, not the DB; prompt/response logs off by default |
| Malicious or faulty upstream | Returns hostile error bodies, stalls, disconnects | Sanitized error mapping, timeouts, retries only before the first byte, circuit breaker, no credential echo |
| Accidental disclosure via telemetry | Secrets or prompts arrive in logs, metrics, traces | Redaction rules in §6; no prompt content in traces; low-cardinality metric labels |

**Trust boundaries:** Internet → Caddy → data plane (`:8080`); private network/VPN → admin listener (`:8081`); gateway → provider APIs; gateway → SQLite volume. Nothing crosses a boundary with a raw credential.

## 3. Key handling

### Client virtual keys

- Generate 32+ random bytes and format as `gw-<base64url>`. Show the secret once at creation; never return it from a list or detail endpoint.
- Store only a SHA-256 hash plus a display prefix (`gw-ab12…`). SHA-256 is appropriate for high-entropy secrets; it is not used for admin passwords.
- Look up hashes through a short-TTL in-memory cache, invalidated immediately on revoke.
- Keys are per-project, may expire, may be revoked, and carry their own rate limits and alias allowlist.
- V2 adds rotation with an overlap window and last-used tracking; the overlap logic is never cut.

### Provider credentials

- Held only in the gateway. Encrypted at rest and never returned by the admin API, including the test-connection endpoint.
- Never sent to clients, never placed in URLs or headers bound for clients, and never included in errors.
- Adapters hold a shared `reqwest::Client`; credentials are applied upstream only.

### Admin credentials and sessions

- Bootstrap admin comes from `ADMIN_BOOTSTRAP_EMAIL` / `ADMIN_BOOTSTRAP_PASSWORD` on first start.
- Passwords hashed with Argon2. Login is throttled; failed attempts are not enumerated in responses.
- Session cookie is `HttpOnly` and `SameSite=Strict`; deploy with `Secure` when TLS is terminated at Caddy. No tokens in `localStorage`.
- Every mutation writes an `audit_log` row. V2 adds scoped admin roles and richer audit views.

## 4. Encryption at rest

- Provider credentials use AES-256-GCM (`aes-gcm`) with a master key from `GATEWAY_MASTER_KEY`. The master key is never stored in the database or committed to the repo.
- The SQLite database lives on a mounted volume; backups use `sqlite3 .backup` or Litestream and must be protected at least as strongly as the volume.
- Full prompt/response logging is off by default. When enabled per project, those rows contain plaintext content and require the same care as the volume.
- V1 does not rotate the master key with zero downtime; changing it requires re-encrypting provider credentials through a short maintenance window ([operations.md §5](operations.md#5-rotating-the-master-key)).
- V2 guardrails add redaction modes; log-only redaction is the default and matched sensitive content is never stored.

## 5. Plane separation and network

- Two listeners: data plane on `:8080` for projects, admin API and built UI on `:8081` for the operator. They share crates but use different auth and middleware stacks.
- The admin listener binds to localhost or a private network by default and is not published by `docker-compose`. Only `:8080` is exposed through Caddy.
- `/metrics` is served on the admin listener only; `/health` and `/ready` exist on both.
- Caddy terminates TLS; the gateway speaks HTTP behind it. Do not expose either listener directly to the Internet.
- Splitting into two binaries later preserves this boundary (ADR 0003).

## 6. Logging rules

Logging mode is `off`, `metadata` (default), or `full` (opt-in per project). The `Authorization` header is redacted in every mode.

**Always logged** (metadata mode): request id, `X-Gateway-Tag` when supplied, project id and key id (identifiers, never secrets), requested alias, final provider and upstream model, token counts, cost, latency, TTFT, status, HTTP status, error kind, per-attempt outcomes, and cache/guardrail decision fields in later versions.

**Logged only in `full` mode:** request body (prompts, tool definitions) and response body (completion or SSE-assembled output).

**Never logged:**

- The `Authorization` header, virtual key secrets, or key prefixes beyond the stored display prefix.
- Provider credentials, `GATEWAY_MASTER_KEY`, admin passwords, or session cookies/tokens.
- Prompt or response content in `off` and `metadata` modes.
- Upstream error bodies that could contain credentials or internal endpoints; errors are sanitized before logging and before being returned to clients.
- Sensitive values matched by V2 guardrails (emails, card numbers, IDs), even when redaction is enabled.
- Raw model input or ID-like values as metric labels, and prompt content in traces.

Operational rules:

- Logs use `tracing` with JSON output and carry `request_id`, project, and alias.
- Metrics labels are limited to low-cardinality values (project, alias, provider, status); never key id, request id, or raw input.
- Traces have a root span per request and a child span per upstream attempt, with no prompt content.
- At V2, guardrail decisions are recorded as `guardrail_action` and `rule_id`, never the matched content.
- Security review and dependency audits (`cargo audit`, `cargo deny`) are part of S17.

## 7. Abuse and reliability controls

- Body-size limits, per-key token-bucket rate limiting, concurrency caps, and monthly budget enforcement with hard cut-off.
- 429 responses include `Retry-After`; budget overrun is a distinct `budget_exceeded` error type.
- Usage logging is best-effort (`try_send` and a bounded channel); dropped events increment a metric. Budget enforcement uses a separate spend counter maintained on the request path, so log loss cannot bypass a cap.
- Config changes swap an immutable snapshot atomically; in-flight requests keep the view they started with.
- Graceful shutdown drains in-flight streams and the log channel before exit.

## 8. Out of scope

- Multi-tenant SaaS isolation, public billing, and high availability in V1.
- Cross-project cache sharing is opt-in in V2; project-scoped keys are the default.
- Multi-instance security (shared rate limits, config propagation, instance identity) is V3 and requires revisiting the trust boundaries above.
