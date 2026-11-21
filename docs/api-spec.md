# LLM Gateway — Data-Plane API Specification

This is the contract for the public, OpenAI-compatible data plane. Clients depend on this document, not on provider SDK internals. The admin API (`/admin/v1/*`) is a separate surface described in [architecture §7](architecture.md#7-http-surfaces).

## 1. Scope and conformance

- Base path: `/v1` on the data-plane listener (`:8080`). Requests are JSON unless the response is an SSE stream.
- Compatibility target: the OpenAI Chat Completions API. An existing client should work by changing only `base_url` and `api_key`.
- The `model` field is an **alias** (`fast`, `smart`), not an upstream model name. See ADR 0004.
- The gateway normalizes a common subset and passes provider-specific fields through where possible; it does not promise every OpenAI field. Section 6 is the authoritative list.
- A request is either accepted fully or rejected with an error object. Silently ignoring a field that changes response semantics is not allowed.

## 2. Endpoints

| Method | Path | Purpose | Auth |
|--------|------|---------|------|
| `POST` | `/v1/chat/completions` | Chat completion, non-streaming or SSE | Virtual key |
| `GET` | `/v1/models` | List aliases available to this key | Virtual key |
| `GET` | `/health` | Process liveness | None |
| `GET` | `/ready` | DB reachable, config loaded, at least one provider enabled | None |

`GET /health` returns `200 {"status":"ok"}`. `GET /ready` returns `200 {"status":"ready"}` or `503 {"status":"not_ready","reason":"..."}`. Both are also available on the admin listener. `/metrics` is admin-only and arrives in V2.

## 3. Authentication and headers

### Request headers

| Header | Required | Rules |
|--------|----------|-------|
| `Authorization` | Yes | `Bearer gw-<base64url>` virtual key. Missing, invalid, revoked, or expired keys get 401. |
| `Content-Type` | Yes | `application/json` |
| `X-Request-Id` | No | 1–128 chars of `[A-Za-z0-9._-]`. Generated as a UUID when absent; echoed on every response and written to logs. |
| `X-Gateway-Tag` | No | Up to 64 chars. Stored on the request log row and used as a usage/query filter. Never affects routing or billing. |
| `X-Gateway-Cache` | No | V2 only: `bypass` or `no-store`. Ignored in V1. |

Provider credentials are never accepted from a client header. The `Authorization` header is always redacted from logs.

### Response headers

| Header | When |
|--------|------|
| `Content-Type: application/json` | All non-streaming responses and errors |
| `Content-Type: text/event-stream` | Streaming responses |
| `X-Request-Id` | Always; mirrors the request or the generated id |
| `Retry-After` | 429 responses; whole seconds until retry is allowed |

Rate-limit headers (`X-RateLimit-*`) are not emitted in V1.

## 4. `POST /v1/chat/completions` — request

### Normalized fields

These fields are parsed, validated, and translated by every provider adapter.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `model` | string | Yes | Alias visible to this key. Unknown → 404; not allowlisted → 403. |
| `messages` | array | Yes | 1+ message objects, ordered. Empty → 400. |
| `stream` | boolean | No | Default `false`. When true, response is SSE (section 6). |
| `max_tokens` | integer ≥ 1 | No | If omitted for an Anthropic-backed alias, the gateway supplies a configured default because Anthropic requires it. |
| `temperature` | number 0–2 | No | Passed through when the upstream supports it. |
| `tools` | array | No | Function tool definitions; mapped to each provider's tool format. |
| `response_format` | object | No | `{"type":"text"}` or `{"type":"json_object"}`. Providers without structured-output support return 400 rather than silently ignoring it. |

Message objects:

| Field | Type | Notes |
|-------|------|-------|
| `role` | enum | `system`, `user`, `assistant`, `tool`. Other roles → 400. |
| `content` | string or null | Required for `system`/`user`. `null` is allowed for `assistant` with `tool_calls`. |
| `name` | string | Optional participant name. |
| `tool_call_id` | string | Required when `role` is `tool`. |
| `tool_calls` | array | Assistant tool calls: `id`, `type: "function"`, `function: {name, arguments}`. Arguments are JSON-encoded strings. |

Tool definition: `{"type":"function","function":{"name":...,"description":...,"parameters":{...}}}` with a JSON Schema `parameters` object.

### Limits

- Request bodies above the configured size limit are rejected with 413 before parsing.
- Only one choice per request is supported (`n` must be 1 or absent).
- Multimodal content parts are not normalized in V1; see section 5.

## 5. OpenAI field support

| Field | Handling | Notes |
|-------|----------|-------|
| `model`, `messages`, `stream`, `max_tokens`, `temperature`, `tools`, `response_format` | **Normalized** | Section 4; translated by every adapter. |
| `top_p`, `stop`, `presence_penalty`, `frequency_penalty`, `seed` | **Passed through** | Copied into `extra`; forwarded only when the selected provider supports them, otherwise ignored with a debug log. |
| `user`, `metadata` | **Passed through** | Forwarded when supported; `user` is not used for gateway accounting (the virtual key is). |
| `tool_choice`, `parallel_tool_calls` | **Passed through** | Forwarded when the provider supports tool-call controls; otherwise ignored. |
| `logit_bias`, `reasoning_effort` | **Passed through** | Provider-specific; unsupported values may produce a provider-level 400, which is returned to the client. |
| `stream_options.include_usage` | **Honored** | The gateway also measures TTFT and usage itself for logging. Other `stream_options` keys are rejected. |
| `n` | **Rejected unless 1** | Multiple choices are not supported. |
| `logprobs`, `top_logprobs` | **Rejected** | 400 `unsupported_field`. |
| `functions` | **Rejected** | Legacy API; use `tools`. |
| `audio`, `modalities`, `prediction`, `service_tier`, `web_search_options` | **Rejected** | 400 `unsupported_field`. |
| Multimodal content parts (`image_url`, `input_audio`) | **Rejected** | 400 `unsupported_field`; content must be a string. |
| Unknown top-level fields | **Ignored** | Logged and counted; forward compatibility with SDKs is preferred over strict rejection. Provider-specific fields should be sent under `extra` when supported by that adapter. |

`extra` is a JSON object of provider-specific values. It is accepted on the request and is the explicit escape hatch for fields not listed above; it is not a way to bypass auth, limits, or alias resolution.

## 6. Responses

### Non-streaming

```json
{
  "id": "req_01J8Z...",
  "object": "chat.completion",
  "created": 1763664000,
  "model": "smart",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Hello!",
      "tool_calls": []
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 12,
    "completion_tokens": 5,
    "total_tokens": 17,
    "prompt_tokens_details": { "cached_tokens": 0 }
  }
}
```

- `id` is the gateway request id, not the upstream id; it equals `X-Request-Id` when one was supplied.
- `model` is the requested alias. The upstream provider/model is recorded in the request log, never exposed here.
- `finish_reason` is `stop`, `length`, `tool_calls`, or `content_filter`.
- `usage.prompt_tokens_details.cached_tokens` is present only when the upstream reports cached input tokens.
- Cost is not part of the OpenAI response; query it through the admin API usage endpoints.

### Streaming (`stream: true`)

Response is `text/event-stream` with OpenAI-style chunks:

```
data: {"id":"req_01J8Z...","object":"chat.completion.chunk","created":1763664000,"model":"smart","choices":[{"index":0,"delta":{"role":"assistant","content":"He"},"finish_reason":null}]}

data: {"id":"req_01J8Z...","object":"chat.completion.chunk","created":1763664000,"model":"smart","choices":[{"index":0,"delta":{"content":"llo"},"finish_reason":null}]}

data: {"id":"req_01J8Z...","object":"chat.completion.chunk","created":1763664000,"model":"smart","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":12,"completion_tokens":5,"total_tokens":17}}

data: [DONE]
```

- `delta.tool_calls` carries tool-call fragments with an `index` for assembly across chunks.
- A final chunk may carry `usage`; the gateway always records usage, TTFT, and status in the request log.
- Retries and fallbacks happen only before the first chunk. After the stream commits, a failure produces a final `data:` event with an `error` object and then terminates the stream **without** `[DONE]`.
- Client disconnect cancels the upstream request; the request is logged as `client_cancelled` with whatever tokens were observed.

## 7. Errors

All errors use the OpenAI-shaped body:

```json
{ "error": { "message": "...", "type": "rate_limit_error", "code": "rate_limit_exceeded" } }
```

`message` is safe to display: it never contains provider credentials, internal paths, or raw upstream bodies.

| Situation | HTTP | `type` | `code` |
|-----------|------|--------|--------|
| Missing/invalid/revoked/expired key | 401 | `authentication_error` | `invalid_api_key` |
| Alias not allowed for key | 403 | `permission_error` | `alias_not_allowed` |
| Unknown alias | 404 | `invalid_request_error` | `unknown_alias` |
| Malformed request, unsupported field, upstream 400 | 400 | `invalid_request_error` | `invalid_request` / `unsupported_field` |
| Body too large | 413 | `invalid_request_error` | `request_too_large` |
| Rate limit exceeded | 429 | `rate_limit_error` | `rate_limit_exceeded` |
| Monthly budget exceeded | 429 | `budget_exceeded` | `budget_exceeded` |
| All upstream targets failed | 502 | `upstream_error` | `upstream_failed` |
| Upstream timeout | 504 | `timeout_error` | `upstream_timeout` |
| Gateway internal error | 500 | `internal_error` | `internal_error` |

Upstream 400/401/429/5xx responses are mapped to this shape; upstream credentials are never echoed. Non-retryable upstream errors are returned immediately, while retryable ones are retried and may fall back before any byte is sent to the client.

## 8. `GET /v1/models`

Lists aliases the key may use:

```json
{
  "object": "list",
  "data": [
    { "id": "fast", "object": "model", "created": 0, "owned_by": "gateway" },
    { "id": "smart", "object": "model", "created": 0, "owned_by": "gateway" }
  ]
}
```

Upstream model names are intentionally not listed.

## 9. Compatibility policy

- `/v1` is the OpenAI-compatible prefix. Breaking changes require a new prefix or an explicit versioned opt-in.
- Adding an optional request field or a response field is not breaking.
- Newly unsupported fields are rejected with `unsupported_field` rather than ignored; document them in section 5 in the same change.
- Contract tests with the official Python and TypeScript SDKs (sprint 7.2) are the acceptance gate for this specification.
