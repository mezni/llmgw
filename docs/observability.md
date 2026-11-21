# Observability

Log fields, metric names, dashboards, and alerts. The short design is in [architecture §11](architecture.md#11-observability); the rules about what must never be logged are in [security.md §6](security.md#6-logging-rules). Metrics and traces arrive in 0.3.0 (V2); JSON logs and health endpoints exist from the start.

## 1. Logs

`tracing` emits JSON with a stable field set. Every data-plane request line carries `request_id`, `project_id`, and `alias`.

| Field | Type | Always? | Meaning |
|-------|------|---------|---------|
| `ts` | RFC 3339 UTC | Yes | Event time |
| `level` | string | Yes | `error`, `warn`, `info`, `debug` |
| `request_id` | string | Yes | From `X-Request-Id` or generated UUID |
| `project_id`, `key_id` | string | Yes (data plane) | Identifiers only, never secrets |
| `alias` | string | Yes (data plane) | Client-facing model alias |
| `provider_id`, `model` | string | Final target | Upstream provider and real model |
| `status`, `http_status`, `error_kind` | string/int | Yes | Outcome and normalized error |
| `latency_ms`, `ttft_ms` | int | On completion | Total time and time to first token |
| `input_tokens`, `output_tokens`, `cached_tokens` | int | When reported | Normalized usage |
| `cost_usd` | number | On completion | Computed from model pricing |
| `attempts` | array | On completion | Per-attempt provider, outcome, latency |
| `tag` | string | When supplied | `X-Gateway-Tag`, stored and searchable |
| `log_mode` | string | Yes | `off`, `metadata`, `full` |
| `span_id`, `trace_id` | string | 0.3.0+ | OpenTelemetry correlation |
| `guardrail_action`, `rule_id` | string | 0.3.0+ | Decision only, never matched content |

`request_body` / `response_body` appear only in `full` mode and are pruned after 7 days. `Authorization`, provider credentials, master keys, admin passwords, session cookies, prompt content in metadata mode, and raw upstream error bodies are never logged.

## 2. Metrics

Prometheus text endpoint `/metrics` on the admin listener only, never on `:8080`. Label values are low cardinality: `project`, `alias`, `provider`, `status`, `kind`, `rule`. No key id, request id, or raw input as a label.

| Metric | Type | Labels | Purpose |
|--------|------|--------|---------|
| `gateway_requests_total` | counter | `project`, `alias`, `provider`, `status` | Volume and error rate |
| `gateway_request_duration_seconds` | histogram | `alias`, `provider` | p50/p95/p99 latency |
| `gateway_ttft_seconds` | histogram | `alias`, `provider` | Streaming responsiveness |
| `gateway_input_tokens_total` | counter | `project`, `alias` | Input usage |
| `gateway_output_tokens_total` | counter | `project`, `alias` | Output usage |
| `gateway_cached_input_tokens_total` | counter | `project`, `alias` | Cache savings |
| `gateway_cost_usd_total` | counter | `project`, `alias`, `provider` | Spend by dimension |
| `gateway_upstream_errors_total` | counter | `provider`, `kind` | Provider health |
| `gateway_retries_total` | counter | `alias`, `provider` | Retry pressure |
| `gateway_fallbacks_total` | counter | `alias`, `provider` | Fallback usage |
| `gateway_circuit_breaker_state` | gauge | `provider` | 0 closed, 1 half-open, 2 open |
| `gateway_budget_exceeded_total` | counter | `project` | Hard caps hit |
| `gateway_rate_limited_total` | counter | `project`, `alias` | Throttling |
| `gateway_key_auth_failures_total` | counter | — | Invalid/revoked key attempts |
| `gateway_log_events_dropped_total` | counter | — | Usage-log channel loss |
| `gateway_cache_hits_total` / `gateway_cache_misses_total` | counter | `alias` | Exact/semantic cache effectiveness (V2) |
| `gateway_guardrail_actions_total` | counter | `rule`, `action` | Redaction/block decisions (V2) |

Traces in 0.3.0+ have one root span per request and one child span per upstream attempt, carrying provider, alias, retry number, and status, but no prompt content. OTLP export is off by default.

## 3. Health endpoints

Both listeners serve `GET /health` (`200 {"status":"ok"}`) and `GET /ready` (`200 {"status":"ready"}` or `503 {"status":"not_ready","reason":"..."}`). Use `/health` for liveness and `/ready` for readiness; a failing `/ready` means DB unreachable, config not loaded, or no provider enabled (see [operations.md §7](operations.md#7-common-failures)).

## 4. Dashboards

| Dashboard | Panels | Audience |
|-----------|--------|----------|
| Gateway overview | Request rate, error rate, p95 latency, TTFT, tokens/min, active aliases | Operator, daily |
| Cost and usage | Spend by project/alias/provider, cached-token ratio, budget burn-down | Owner, weekly |
| Provider health | Error rate and latency by provider, retries, fallbacks, circuit-breaker state | Operator, incident |
| Gateway internals | Log drops, WAL size proxy, auth failures, retention job duration | Operator, deep debug |

All dashboards default to a 30-day window with UTC labels. Charts mirror the admin dashboard but use metrics instead of SQL so they keep working when log mode is `off`.

## 5. Alerts that matter

| Alert | Condition | Severity | Action |
|-------|-----------|----------|--------|
| Gateway not ready | `/ready` non-200 for 2 minutes | Page | Check DB and provider config; see runbook |
| High error rate | `5xx` ratio > 5% for 5 minutes | Page | Inspect `upstream_errors_total` and attempt logs |
| Latency regression | p95 > 10s for 10 minutes | Page | Check provider latency and timeout settings |
| Poor TTFT | p95 TTFT > 3s for 15 minutes | Ticket | Tune alias/provider timeout or upstream |
| Budget burn | Project spend > 80% of monthly cap | Ticket | Notify project owner; cap at 100% with 429 |
| Budget cap hit | `budget_exceeded_total` increases | Ticket | Confirm cap is intentional |
| Provider circuit open | `gateway_circuit_breaker_state == 2` for 5 minutes | Ticket | Test connection; switch fallback order |
| Log drops | `gateway_log_events_dropped_total` grows for 5 minutes | Ticket | Reduce log mode or widen writer channel; budgets stay correct |
| Auth failures spike | `key_auth_failures_total` jumps | Ticket | Check for leaked key; revoke and reissue |
| Disk pressure | Data volume > 80% | Page | Check WAL, backups, retention job |

Do not alert on individual 429s, single upstream timeouts, cache misses, or prompt content. Low-volume, expected failures are visible on dashboards only.
