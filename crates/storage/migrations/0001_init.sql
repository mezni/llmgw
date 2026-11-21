CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  monthly_budget_usd REAL,
  monthly_spend_usd REAL NOT NULL DEFAULT 0,
  spend_period TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE api_keys (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  prefix TEXT NOT NULL,
  key_hash BLOB NOT NULL UNIQUE,
  rpm_limit INTEGER,
  tpm_limit INTEGER,
  expires_at TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX idx_api_keys_project ON api_keys(project_id);

CREATE TABLE requests (
  id TEXT PRIMARY KEY,
  ts TEXT NOT NULL,
  project_id TEXT NOT NULL,
  key_id TEXT NOT NULL,
  alias TEXT NOT NULL,
  tag TEXT,
  provider_id TEXT,
  model TEXT,
  stream INTEGER NOT NULL,
  input_tokens INTEGER,
  output_tokens INTEGER,
  cached_tokens INTEGER,
  cost_usd REAL,
  latency_ms INTEGER,
  ttft_ms INTEGER,
  status TEXT NOT NULL,
  http_status INTEGER,
  error_kind TEXT,
  attempts_json TEXT,
  request_body TEXT,
  response_body TEXT
);

CREATE INDEX idx_requests_project_ts ON requests(project_id, ts);
CREATE INDEX idx_requests_alias_ts ON requests(alias, ts);
CREATE INDEX idx_requests_ts ON requests(ts);
CREATE INDEX idx_requests_status_ts ON requests(status, ts);
