# Provider compatibility

What the gateway actually supports per upstream, as of V1. The [data-plane API spec](api-spec.md) is the client contract; this matrix describes how that contract maps onto each provider. When a row says "rejected", the gateway returns `400 unsupported_field` before calling the provider.

Legend: **Supported** (normalized and tested), **Partial** (works with caveats), **No (V1)** (upstream may support it, the gateway does not), **Rejected** (gateway refuses the request).

## Capability matrix

| Capability | OpenAI-compatible | Anthropic | Other OpenAI-compatible |
|------------|-------------------|-----------|-------------------------|
| Non-streaming chat | Supported | Supported | Supported when the vendor follows the OpenAI shape |
| Streaming (SSE) | Supported; `[DONE]` terminator | Supported; Anthropic events translated to OpenAI chunks | Partial; some vendors differ in framing or event fields |
| Tools / function calling | Supported; `tool_calls` fragments assembled during streaming | Supported; `parameters` ↔ `input_schema`, tool results via `role: tool` | Partial; depends on tool schema fidelity |
| Structured output (`response_format`) | Partial: `text` and `json_object` where the model supports them; `json_schema` is not normalized | No (V1): `json_object` returns 400 rather than being silently ignored | Partial; verify with a recorded fixture |
| Images / multimodal input | Rejected in V1 (`content` must be a string) | Rejected in V1 | Rejected in V1 |
| Usage reporting | Supported: input/output tokens; cached tokens via `prompt_tokens_details.cached_tokens` | Supported: `input_tokens`/`output_tokens`; cache reads normalized to `cached_tokens` | Partial: some vendors omit usage, especially in streams |
| Cost attribution | Supported via the `models` pricing row | Supported via the `models` pricing row | Supported when pricing is configured |
| Model listing (`/v1/models`) | Aliases only; upstream names are never exposed | Same | Same |
| Native `/v1/messages` | Not applicable | Planned V3 (S12) | Not applicable |
| Embeddings | Planned V3 (S12) via `Provider::embed` | Not planned | Depends on adapter |

## Known quirks

### OpenAI-compatible

- Newer reasoning models may prefer `max_completion_tokens` over `max_tokens`, and some reject `temperature`. The adapter maps `max_tokens` where needed; an upstream rejection is returned as a normalized 400 rather than retried.
- `usage` in a streamed response is only emitted by some providers when `stream_options.include_usage` is set. The gateway records usage from its own stream tap regardless, and also forwards the option when supported.
- `model` in responses may be a deployment name rather than the requested alias. The gateway always returns the client's alias and records the real upstream model in the request log.
- "OpenAI-compatible" vendors vary in SSE comments, `[DONE]` handling, and error bodies. Deviation beyond a small adapter shim warrants its own provider kind.

### Anthropic

- `max_tokens` is required. When the client omits it, the gateway supplies a configured default before calling the provider.
- `system` is a separate top-level field upstream; the adapter extracts `system` messages from `messages`.
- Tool definitions map between OpenAI `function.parameters` and Anthropic `input_schema`; tool result blocks are mapped back to `role: tool` messages.
- Streaming uses `message_start`, `content_block_start`, `content_block_delta`, `message_delta`, and `message_stop` events. Tool-call fragments require index-based assembly before being emitted as OpenAI-style deltas.
- Cached input accounting uses `cache_read_input_tokens` and `cache_creation_input_tokens`; these normalize to `cached_tokens` in the gateway's `Usage` and are exposed as `prompt_tokens_details.cached_tokens` in OpenAI-shaped responses.
- Provider-specific features such as extended thinking blocks and cache-control directives are not part of the V1 normalized contract. Unsupported fields are documented here and rejected or passed through per [api-spec §5](api-spec.md#5-openai-field-support).

### Other OpenAI-compatible providers

- Configured with a base URL and encrypted credentials through the admin API; no new code is required if the wire format matches.
- A failing test-connection result is reported in the admin UI; credentials are never echoed back.
- Treat unknown vendors as untrusted until their streaming and error behavior is covered by recorded fixtures.

## Test obligations

- Every adapter change needs recorded request/response fixtures, including fragmented and malformed SSE.
- Integration tests exercise the shared scenario set against the mock provider: success, slow, 429 with `Retry-After`, 500, mid-stream cut, malformed SSE.
- Contract tests use the official OpenAI (Python, TypeScript) and Anthropic SDKs; streaming and error shapes are part of the gate.
- Real-provider calls are deliberate, use small prompts, and are never required for CI to pass.
- Add or update this matrix in the same change that changes provider support. See [conventions.md](conventions.md) for the definition of done.

## Adding a provider

Implement `Provider` in the `providers` crate, register its `kind`, add fixtures and a mock scenario, then add a column here. No other layer changes (see [architecture §16](architecture.md#16-extension-points)).
