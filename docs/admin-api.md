# Admin API

Control-plane contract for the gateway. The public OpenAI-compatible data plane is specified separately in [api-spec.md](api-spec.md); the React client that consumes this API is described in [admin-ui.md](admin-ui.md).

## 1. Scope

- Listener: admin port `:8081` only. Never exposed on the public data-plane port.
- Base path: `/admin/v1`. All bodies are `application/json`; request and response field names are `snake_case`.
- Auth: email/password login with an `HttpOnly`, `SameSite=Strict` session cookie. No bearer tokens, no `localStorage` tokens.
- Bootstrap admin comes from `ADMIN_BOOTSTRAP_EMAIL` / `ADMIN_BOOTSTRAP_PASSWORD` on first start (see [config-reference.md](config-reference.md)).
- Every mutating call writes one `audit_log` row and triggers an atomic config-snapshot rebuild when routing, pricing, keys, or limits change.
- Provider credentials are write-only: they are encrypted at rest and never returned by any endpoint, including test-connection results.

## 2. Conventions

- IDs are server-generated UUIDs in responses. Clients may send an `id` on create for idempotent retries; a duplicate ID returns the existing resource with `200`.
- Lists use `?limit=…&cursor=…` with `limit` between 1 and 200 (default 50). The cursor is opaque and encodes the last `(ts, id)` pair.
- Time ranges use RFC 3339 `from` / `to` parameters. Missing `to` means "now". Bucket values are `hour` or `day`.
- Soft concepts are expressed explicitly: `revoked_at`, `expires_at`, `enabled`. Deletes are real deletes except for request history.
- Validation errors return `422` with a `details` map keyed by field name.
- `/health` and `/ready` are also available here; `/metrics` is admin-only and arrives in V2.

## 3. Errors

```json
{
  "error": {
    "code": "validation_failed",
    "message": "Budget must be greater than zero.",
    "details": { "monthly_budget_usd": "must be > 0" }
  }
}
```

| Situation | HTTP | `code` |
|-----------|------|--------|
| No/invalid session | 401 | `unauthenticated` |
| Authenticated but insufficient role (V2) | 403 | `forbidden` |
| Unknown resource | 404 | `not_found` |
| Malformed JSON / invalid field | 400 | `bad_request` |
| Validation failure | 422 | `validation_failed` |
| Duplicate name | 409 | `conflict` |
| Upstream provider unreachable on test | 502 | `provider_unreachable` |
| Unexpected failure | 500 | `internal_error` |

`message` is safe to display. Raw `sqlx` errors, provider bodies, stack traces, credentials, and internal paths are never returned (see [security.md §6](security.md#6-logging-rules)).

## 4. Endpoints

### Auth

| Method | Path | Body | Response |
|--------|------|------|----------|
| `POST` | `/admin/v1/auth/login` | `{email, password}` | `204`; sets session cookie |
| `POST` | `/admin/v1/auth/logout` | — | `204`; clears cookie |

Failed logins are throttled and return the same 401 regardless of whether the email exists.

### Projects and keys

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/admin/v1/projects` | List projects with current month spend |
| `POST` | `/admin/v1/projects` | Create `{name, monthly_budget_usd?}` |
| `GET` | `/admin/v1/projects/{id}` | Detail including spend and key count |
| `PATCH` | `/admin/v1/projects/{id}` | Update `name` / `monthly_budget_usd` |
| `DELETE` | `/admin/v1/projects/{id}` | Delete project and its keys/allowlists |
| `GET` | `/admin/v1/projects/{id}/keys` | List key prefixes, limits, expiry, status |
| `POST` | `/admin/v1/projects/{id}/keys` | Create `{rpm_limit?, tpm_limit?, expires_at?, allowed_aliases?}`; returns the secret **once** |
| `PATCH` | `/admin/v1/keys/{id}` | Update limits or expiry |
| `DELETE` | `/admin/v1/keys/{id}` | Revoke (sets `revoked_at`); later purged per [data-model §5](data-model.md#5-retention) |

Create-key response:

```json
{
  "id": "01J8Z…",
  "prefix": "gw-ab12…",
  "secret": "gw-ab12…full-secret",
  "created_at": "2026-11-20T14:03:11Z"
}
```

A subsequent `GET` never includes `secret`. V2 adds rotation with an overlap window.

### Providers, models, and aliases

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/admin/v1/providers` | List providers without credentials |
| `POST` | `/admin/v1/providers` | Create `{id, kind, base_url, credentials, timeout_ms, enabled?}` |
| `PATCH` | `/admin/v1/providers/{id}` | Update URL, credentials, timeout, enabled |
| `POST` | `/admin/v1/providers/{id}/test` | Make a tiny upstream request; returns latency and status only |
| `GET` | `/admin/v1/models` | List models and pricing |
| `POST` | `/admin/v1/models` | Create `{id, provider_id, upstream_name, prices…}` |
| `PATCH` | `/admin/v1/models/{id}` | Update pricing or upstream name |
| `GET` | `/admin/v1/aliases` | List aliases with ordered targets |
| `POST` | `/admin/v1/aliases` | Create `{id, name, targets[], max_attempts?, timeout_ms?}` |
| `PATCH` | `/admin/v1/aliases/{id}` | Reorder targets, change attempt/timeout settings |
| `DELETE` | `/admin/v1/aliases/{id}` | Delete alias and its targets |

`targets` is an ordered array of model IDs; index 0 is primary, the rest are fallbacks (ADR 0004). Changes take effect for new requests only; in-flight requests keep their snapshot.

### Usage, requests, and audit

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/admin/v1/usage` | Aggregate by `bucket`, filtered by `from`, `to`, `project_id`, `alias`, `model`, `tag` |
| `GET` | `/admin/v1/requests` | Paginated log search; filters `project_id`, `key_id`, `alias`, `status`, `tag`, time range |
| `GET` | `/admin/v1/requests/{id}` | Request detail including per-attempt outcomes and logged bodies when available |
| `GET` | `/admin/v1/audit` | Paginated admin action list |

Usage rows contain `bucket`, `requests`, `input_tokens`, `output_tokens`, `cached_tokens`, `cost_usd`, `error_rate`, `p50_latency_ms`, and `p95_latency_ms`. Bodies are only present in request detail when full logging was enabled and the 7-day content window has not elapsed.

## 5. OpenAPI spec and generated client

The machine-readable contract is [openapi.json](openapi.json). Until sprint task 5.11 lands it is a reviewed draft; after that it is generated output from `utoipa` annotations on the admin handlers and must not be hand-edited.

Generation and consumption:

```bash
cargo run -p admin-api --bin export-openapi > docs/openapi.json
npx openapi-typescript docs/openapi.json -o admin-ui/src/api/schema.d.ts
```

The React app uses `openapi-fetch` with `credentials: 'same-origin'` and the generated `paths` types. CI regenerates both artifacts and fails on drift:

```bash
git diff --exit-code -- docs/openapi.json admin-ui/src/api/schema.d.ts
```

Task 5.11 (OpenAPI export) is a hard dependency for S6 and is never cut: the UI cannot be wired to a contract it cannot type-check against (ADR 0005). Adding or changing an endpoint requires updating this document, the generated spec, and any affected UI query in the same change.

## 6. Testing

- Repository tests cover each handler's validation and error mapping; no test logs a secret.
- Integration tests use the mock provider and a fresh SQLite database per test.
- Contract tests snapshot the OpenAPI document so a handler change cannot silently drift from the spec.
- Playwright smoke flows in S6 exercise login, project/key creation, and the dashboard against the built app (see [test-plan.md](test-plan.md)).
