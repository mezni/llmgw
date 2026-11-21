# Configuration reference

Every setting the gateway reads, its default, and whether changing it requires a restart. Static configuration is TOML with environment overrides; dynamic configuration lives in SQLite and is changed through the [admin API](admin-api.md) without a restart (see [architecture §8](architecture.md#8-configuration-and-hot-reload)).

Precedence is **environment → TOML → built-in default**. Secrets are never written in TOML: the file may name an environment variable, but the value is supplied by the runtime. Never commit `.env` files.

> Current S1–S3 implementation: this reference describes the target V1 surface. The local mock-only gateway does not require `GATEWAY_MASTER_KEY` or the bootstrap admin variables; data-plane auth uses the CLI-managed virtual keys in §5. Admin API/UI configuration sections remain planned work.

## 1. Environment variables

| Variable | Default | Purpose | Restart? |
|----------|---------|---------|----------|
| `GATEWAY_CONFIG` | `./config.toml` | Path to the TOML file | Yes |
| `DATABASE_URL` | `sqlite://./gateway.db` | SQLite connection URL | Yes |
| `GATEWAY_MASTER_KEY` | — (required) | Base64-encoded 32-byte AES-256-GCM master key for provider credentials | Yes |
| `GATEWAY_MASTER_KEY_PREVIOUS` | unset | Previous master key, read-only during rotation (see [operations.md](operations.md#5-rotating-the-master-key)) | Yes |
| `ADMIN_BOOTSTRAP_EMAIL` | — (required on first start) | Email of the initial admin; ignored after `admin_users` has a row | Yes |
| `ADMIN_BOOTSTRAP_PASSWORD` | — (required on first start) | Initial admin password (Argon2-hashed on insert) | Yes |
| `ADMIN_SESSION_TTL_HOURS` | `168` | Session cookie lifetime in hours | Yes |
| `GATEWAY_DATA_LISTEN` | `0.0.0.0:8080` | Public data-plane bind address | Yes |
| `GATEWAY_ADMIN_LISTEN` | `127.0.0.1:8081` | Admin API/UI bind address; keep private | Yes |
| `RUST_LOG` | `info` | `tracing` filter directive, e.g. `info,gateway=debug` | Yes |
| `GATEWAY_LOG_JSON` | `true` | JSON log output instead of human-readable text | Yes |
| `GATEWAY_LOG_MODE` | `metadata` | `off`, `metadata`, or `full` (see [security.md §6](security.md#6-logging-rules)) | Yes |
| `GATEWAY_DEFAULT_TIMEOUT_MS` | `30000` | Upstream timeout when no provider/alias override exists | Yes |
| `GATEWAY_MAX_BODY_BYTES` | `1048576` | Request body limit (1 MiB) | Yes |
| `GATEWAY_UPSTREAM_BASE_URL` | unset | OpenAI-compatible upstream base URL; when unset, S1 starts an in-process mock provider | Yes |
| `GATEWAY_UPSTREAM_API_KEY` | unset | Credential sent to that upstream; not needed for the mock | Yes |
| `GATEWAY_ANTHROPIC_BASE_URL` | `https://api.anthropic.com/v1` | Anthropic Messages API base URL; used only when an API key is set | Yes |
| `GATEWAY_ANTHROPIC_API_KEY` | unset | Anthropic credential; model names beginning with `claude` route to Anthropic when set | Yes |
| `GATEWAY_LOG_RETENTION_DAYS` | `90` | `requests` metadata retention | Yes (job rereads on next tick) |
| `GATEWAY_BODY_RETENTION_DAYS` | `7` | `request_body` / `response_body` retention in full mode | Yes (job rereads on next tick) |
| `GATEWAY_AUDIT_RETENTION_DAYS` | `365` | `audit_log` retention | Yes (job rereads on next tick) |
| `GATEWAY_METRICS_ENABLED` | `false` | V2: expose `/metrics` on the admin listener | Yes |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | unset | V2: OTLP trace exporter endpoint; unset disables export | Yes |

Any unknown `GATEWAY_*` variable is ignored with a startup warning so a typo does not silently change behavior. Anthropic selection is temporary: with an API key set, requests for models whose name begins with `claude` use `GATEWAY_ANTHROPIC_BASE_URL`; all other requests use the OpenAI-compatible provider. Aliases replace this selector in S4.

## 2. TOML settings

```toml
# config.toml — no secrets go in this file
[server]
data_listen = "0.0.0.0:8080"
admin_listen = "127.0.0.1:8081"

[database]
url = "sqlite://./gateway.db"

[security]
master_key_env = "GATEWAY_MASTER_KEY"
previous_master_key_env = "GATEWAY_MASTER_KEY_PREVIOUS"
session_ttl_hours = 168
bootstrap_admin_email_env = "ADMIN_BOOTSTRAP_EMAIL"
bootstrap_admin_password_env = "ADMIN_BOOTSTRAP_PASSWORD"

[logging]
mode = "metadata"      # off | metadata | full
level = "info"
json = true

[limits]
default_timeout_ms = 30000
max_body_bytes = 1048576

[retention]
requests_days = 90
bodies_days = 7
audit_days = 365

[observability]        # V2
metrics_enabled = false
otlp_endpoint = ""
```

| TOML key | Environment override | Default | Restart? |
|----------|----------------------|---------|----------|
| `server.data_listen` | `GATEWAY_DATA_LISTEN` | `0.0.0.0:8080` | Yes |
| `server.admin_listen` | `GATEWAY_ADMIN_LISTEN` | `127.0.0.1:8081` | Yes |
| `database.url` | `DATABASE_URL` | `sqlite://./gateway.db` | Yes |
| `security.master_key_env` | — | `GATEWAY_MASTER_KEY` | Yes |
| `security.previous_master_key_env` | — | `GATEWAY_MASTER_KEY_PREVIOUS` | Yes |
| `security.session_ttl_hours` | `ADMIN_SESSION_TTL_HOURS` | `168` | Yes |
| `security.bootstrap_admin_email_env` | — | `ADMIN_BOOTSTRAP_EMAIL` | Yes |
| `security.bootstrap_admin_password_env` | — | `ADMIN_BOOTSTRAP_PASSWORD` | Yes |
| `logging.mode` | `GATEWAY_LOG_MODE` | `metadata` | Yes |
| `logging.level` | `RUST_LOG` | `info` | Yes |
| `logging.json` | `GATEWAY_LOG_JSON` | `true` | Yes |
| `limits.default_timeout_ms` | `GATEWAY_DEFAULT_TIMEOUT_MS` | `30000` | Yes |
| `limits.max_body_bytes` | `GATEWAY_MAX_BODY_BYTES` | `1048576` | Yes |
| `retention.requests_days` | `GATEWAY_LOG_RETENTION_DAYS` | `90` | Yes |
| `retention.bodies_days` | `GATEWAY_BODY_RETENTION_DAYS` | `7` | Yes |
| `retention.audit_days` | `GATEWAY_AUDIT_RETENTION_DAYS` | `365` | Yes |
| `observability.metrics_enabled` | `GATEWAY_METRICS_ENABLED` | `false` | Yes |
| `observability.otlp_endpoint` | `OTEL_EXPORTER_OTLP_ENDPOINT` | unset | Yes |

## 3. Dynamic configuration (no restart)

These are stored in SQLite and changed through `/admin/v1/*`. A mutation writes the DB, rebuilds the immutable `ConfigSnapshot`, and swaps it atomically; in-flight requests keep the old view.

| Setting | Admin endpoint |
|---------|----------------|
| Providers, base URLs, credentials, timeouts, enabled flag | `POST/PATCH /admin/v1/providers` |
| Models and pricing | `POST/PATCH /admin/v1/models` |
| Aliases, target order, attempts, alias timeout | `POST/PATCH /admin/v1/aliases` |
| Project budgets, key limits, expiry, allowlists | `/admin/v1/projects`, `/admin/v1/keys` |
| V2 cache/guardrail settings | `PATCH /admin/v1/aliases`, guardrail endpoints |

Changing a value here does **not** require a restart and is audited. Changing any value in §1 or §2 does.

## 4. Validation at startup

The process refuses to start when:

- `GATEWAY_MASTER_KEY` is missing or is not exactly 32 bytes after base64 decoding.
- The admin listener equals the data listener, or the data listener is bound to a loopback address while the admin listener is public.
- `logging.mode` is not one of `off`, `metadata`, `full`.
- `retention.bodies_days > retention.requests_days`.
- A TOML file is present but malformed.

Warnings (not failures) are emitted for unknown keys, unset bootstrap credentials after the first run, and V2 settings supplied on a V1 build.

## 5. Current CLI key management (S1–S3)

Virtual keys are stored in SQLite as SHA-256 hashes with a display prefix; the full secret is printed once at creation.

```bash
cargo run -p server -- key create --project NAME
cargo run -p server -- key list --project NAME
cargo run -p server -- key revoke --project NAME --key-id ID
```

Use the printed secret as `Authorization: Bearer <secret>` on `/v1/*` data-plane requests. `/health` and `/ready` do not require a key.
