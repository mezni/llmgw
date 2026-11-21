# Limitations

Deliberate non-goals and V1 gaps. This is the short answer to "does it do X?"—see [provider-compatibility.md](provider-compatibility.md) for per-provider capability details and [architecture §16](architecture.md#16-extension-points) for planned extensions.

## Deliberately unsupported

- **Images and multimodal input.** `content` must be a string; image parts are rejected with `unsupported_field` even when the upstream supports them.
- **`response_format: json_schema`.** V1 normalizes only `text` and `json_object`; providers without structured output return 400 rather than being silently ignored.
- **Client-visible model names.** Clients use aliases only; upstream names stay in the gateway (ADR 0004).
- **Provider credentials for clients.** Provider keys never leave the gateway and are never returned by the admin API.
- **Retries or fallbacks after the first streamed byte.** Once the stream commits, a failure becomes a final error event without `[DONE]`.
- **Prompt/response bodies in logs by default.** `metadata` is the default mode; `full` is opt-in and bodies expire after 7 days.
- **Cross-project cache sharing.** Caches are project-scoped by default; sharing is opt-in in 0.3.0+.
- **RBAC and SSO.** V1 has a single admin role; scoped roles and external identity providers are not part of V1.
- **CORS and second frontend deployments.** The UI is same-origin on the admin listener; there is no separate web host.
- **Downgrades.** Migrations are forward-only; roll forward or restore a backup.
- **OpenAI Responses API, Assistants, batch, fine-tuning, moderation, files, and audio.** V1 implements Chat Completions only; embeddings arrive in 0.4.0.
- **Native Anthropic `/v1/messages`.** Requested in 0.4.0; V1 uses `/v1/chat/completions` translation.
- **Multi-tenant SaaS, public billing, invoicing, and webhooks.** This is a self-hosted gateway for one operator.

## Not in V1 yet

| Capability | Version |
|------------|---------|
| Prometheus `/metrics`, OTLP traces | 0.3.0 |
| Exact-match cache, `X-Gateway-Cache` | 0.3.0 |
| Guardrails and PII redaction | 0.3.0 |
| Key rotation with overlap | 0.3.0 |
| Richer audit views and roles | 0.3.0 |
| `/v1/embeddings` and native `/v1/messages` | 0.4.0 |
| A/B and canary routing | 0.4.0 |
| Postgres backend and migration tool | 0.4.0 |
| Multi-instance rate limits and config propagation | 0.4.0 |
| Semantic caching | 0.4.0 |

## Known single-instance constraints

V1 and V2 run one process. In-memory token buckets, the key cache, and the config snapshot are per-instance; horizontal scaling without shared state would give inconsistent limits. Multi-instance support is 0.4.0. SQLite WAL plus bounded writers handles a single instance well; it is not a shared database for several gateway processes.

## Documented trade-offs

- `REAL` USD columns are rounded at the API boundary; integer minor units are a later migration concern ([data-model.md §1](data-model.md#1-conventions)).
- V1 does not rotate the master key with zero downtime; the dual-read rotation procedure needs a short maintenance window ([operations.md §5](operations.md#5-rotating-the-master-key)).
- Dashboard time series are computed from `requests`; rollup tables are added only when measured query time justifies them ([data-model.md §3](data-model.md#3-indexes)).
- The admin UI is functional over decorative, with no dark mode or mobile-first layout in V1 ([admin-ui.md §6](admin-ui.md#6-design-basics)).
