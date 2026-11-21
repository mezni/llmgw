# Operations runbook

Day-two procedures for a self-hosted gateway: deploying, backing up, rotating the master key, upgrading, and diagnosing failures. Settings referenced here are defined in [config-reference.md](config-reference.md); retention in [data-model.md §5](data-model.md#5-retention); alerts in [observability.md](observability.md).

## 1. Deploying

1. Create a directory with `docker-compose.yml`, `config.toml`, and a `.env` containing `GATEWAY_MASTER_KEY`, `ADMIN_BOOTSTRAP_EMAIL`, and `ADMIN_BOOTSTRAP_PASSWORD`. Do not commit the `.env`.
2. Start with `docker compose up -d`. The multi-stage image builds the Vite app and the Rust binary; one container serves the admin API and built UI on `:8081` and the data plane on `:8080` (S7 task 7.4).
3. Terminate TLS at Caddy and proxy only `:8080` publicly. Keep `:8081` on localhost or a private network; `/metrics` lives there in 0.3.0+.
4. Verify `GET /health` returns `{"status":"ok"}` and `GET /ready` returns `{"status":"ready"}`. A `503` on `/ready` means migrations did not complete or no provider is enabled.
5. Log in at `https://<host>/` with the bootstrap credentials, change the password, add provider credentials, create aliases, and issue a project key.
6. Point a client at the gateway with only `base_url` and `api_key` changed (see [api-spec.md](api-spec.md)).

Mount a named volume for the SQLite database. The container runs as a non-root user and the master key is supplied only through the environment.

## 2. Backups and restore

**Backup (nightly, keep 30 days):**

```bash
sqlite3 /var/lib/gateway/gateway.db \
  ".backup '/backup/gateway-$(date +%F).db'"
```

Use `.backup`, not `cp`, while the gateway is running; WAL mode means the file on disk can lag. Store the backup separately from `GATEWAY_MASTER_KEY`—a backup without the key cannot decrypt provider credentials.

**Restore:**

1. Stop the gateway: `docker compose down`.
2. Verify the backup: `sqlite3 backup.db "PRAGMA integrity_check;"` and spot-check row counts.
3. Replace the database file and remove any stale `-wal` / `-shm` files.
4. Ensure the same `GATEWAY_MASTER_KEY` is in the environment. A different key makes existing `credentials_enc` unreadable.
5. Start the gateway; `sqlx migrate run` runs on boot. Confirm `/ready` is 200 and a known alias returns a completion.
6. Check the admin audit log for the most recent entry timestamp to confirm you restored the intended point in time.

Rehearse a full restore monthly and after any upgrade (S7 task 7.5).

## 3. Routine checks

| Check | Command / signal | Expected |
|-------|------------------|----------|
| Liveness | `curl localhost:8080/health` | `200 {"status":"ok"}` |
| Readiness | `curl localhost:8081/ready` | `200 {"status":"ready"}` |
| Disk | `df -h` on the data volume | Below 80% |
| WAL size | `ls -lh gateway.db-wal` | Bounded; checkpoint occurs automatically |
| Integrity | `sqlite3 gateway.db "PRAGMA integrity_check;"` | `ok` |
| Log pipeline | metric `gateway_log_events_dropped_total` | Not growing |
| Retention | row counts by day in `requests` | Oldest row within configured window |
| Config freshness | change in admin UI reflected in next request | No restart required |

## 4. Non-secret config changes

Settings in [config-reference.md §1–2](config-reference.md#1-environment-variables) require a restart. Dynamic settings (providers, models, aliases, budgets, keys) are changed through the admin API and take effect for new requests via the atomic snapshot swap; in-flight requests keep the old view. Always confirm a change with the admin audit log and a test completion.

## 5. Rotating the master key

Provider credentials are encrypted with `GATEWAY_MASTER_KEY`. V1 rotates in place with a dual-read window; plan a short maintenance window.

1. Back up the database and record the old key securely.
2. Set `GATEWAY_MASTER_KEY_PREVIOUS=<old key>` and `GATEWAY_MASTER_KEY=<new 32-byte base64 key>` in the environment.
3. Restart. Decryption tries the new key first and falls back to the previous key; new writes use the new key.
4. Run `cargo run --bin gateway -- rotate-keys` (or the packaged `gateway rotate-keys`). It re-encrypts every `providers.credentials_enc` row and reports counts; it is idempotent.
5. Verify zero failures, run a provider test from the admin UI, and take a fresh backup.
6. Remove `GATEWAY_MASTER_KEY_PREVIOUS`, restart, and confirm all providers still work.

If the previous key is removed too early, credentials not yet re-encrypted are unrecoverable—restore from backup and repeat. Do not rotate during a provider test or an in-flight config mutation.

## 6. Upgrading

Versions follow [CHANGELOG.md](../CHANGELOG.md): 0.1.0 (MVP), 0.2.0 (V1), 0.3.0 (V2), 0.4.0 (V3).

1. Read the changelog entry for the target version, especially `Changed` and migration notes.
2. Take a backup and record the current version.
3. `docker compose pull && docker compose up -d`. Migrations run on boot; they are forward-only.
4. Check `/ready`, then run a test completion through one alias and a streaming completion through another.
5. For 0.4.0 SQLite → Postgres, use the dedicated migration tool with dry-run, row counts, and checksum verification (S14). Do not attempt it in place.
6. Downgrades are not supported. Roll forward or restore the backup.

Plan a 0.2.0 → 0.3.0 upgrade after the 0.3.0 retention and redaction defaults are confirmed; guardrail changes can alter what is stored.

## 7. Common failures

| Symptom | Likely cause | Diagnosis and fix |
|---------|--------------|-------------------|
| All clients get 401 | Key revoked/expired, or key hash cache stale | Check `api_keys.revoked_at` / `expires_at`; revoke cache invalidates on write. Issue a new key. |
| 502 `upstream_failed` | Provider down, bad credentials, or all fallbacks exhausted | Run the admin test-connection; check provider `enabled`, base URL, and encrypted credential rotation. |
| 504 `upstream_timeout` | Alias/provider timeout too low or upstream slow | Compare `ttft_ms` and `latency_ms` in the request log; raise `timeout_ms` via admin API. |
| 429 `budget_exceeded` | Monthly spend counter reached the cap | Check `projects.monthly_spend_usd` and `spend_period`; raise budget with `PATCH /admin/v1/projects/{id}`. |
| `/ready` 503 | DB unreachable or no enabled provider | Check volume mount and `DATABASE_URL`; enable a provider in the admin UI. |
| 500 `internal_error` with DB lock errors | SQLite writer contention or full disk | Check `df -h`, WAL size, and concurrent admin mutations; WAL plus bounded writers normally prevents this. |
| Dashboard empty | Log mode `off`, wrong time range, or retention purge | Check `GATEWAY_LOG_MODE` and `requests` row counts; content bodies expire after 7 days. |
| Config change not taking effect | In-flight snapshot or stale generated UI client | New requests use the new snapshot; regenerate the OpenAPI client and hard-refresh the UI. |
| Stream ends without `[DONE]` | Upstream cut or malformed SSE; final event carries an error | Inspect `attempts_json` and adapter fixtures; do not retry after the first chunk. |
| `gateway_log_events_dropped_total` grows | Log channel saturated | Reduce `GATEWAY_LOG_MODE`, enlarge the channel, or check writer latency; budgets remain correct. |
| Provider test fails after key rotation | New key not applied or previous key removed early | Re-run `rotate-keys`; re-enter the credential through the admin UI if needed. |
| Master key lost | No backup of the key | Provider credentials cannot be decrypted; restore from backup with the original key or re-enter all credentials. |

Escalate by capturing `request_id`, timestamp, alias, provider, and status from the log line; see [observability.md](observability.md) for the field list. Never paste credentials or prompt bodies into an issue.
