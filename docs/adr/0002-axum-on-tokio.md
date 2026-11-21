# ADR 0002: Build the HTTP servers with axum on tokio

- Status: Accepted
- Date: 2026-11-20
- Related: ADR 0003 (separate planes), architecture §4 and §15

## Context

Both HTTP surfaces need typed request parsing, low-overhead streaming responses, graceful shutdown, and middleware for request IDs, auth, rate limiting, and budgets. The project is also a learning exercise, so the framework should make async and cancellation behavior explicit rather than hiding it.

## Decision

Use `axum` on `tokio` for both the data plane and admin API. Implement cross-cutting concerns as `tower` layers and share them where possible. Hand streaming response bodies directly to axum so upstream SSE bytes are not fully buffered; rely on stream drop to cancel the upstream request. Generate the OpenAPI spec with `utoipa`.

## Alternatives considered

- **actix-web:** capable, but its actor/runtime model and middleware abstractions would add concepts unrelated to the learning goals.
- **warp:** filter composition is less ergonomic for a large set of admin CRUD routes.
- **Raw hyper:** maximum control, but would require hand-writing routing, extractors, and error mapping.

## Consequences

- Middleware is independently testable as tower layers and composes in a clear order.
- Async and cancellation semantics stay visible, which supports correct streaming and shutdown behavior.
- `reqwest` and axum both use tokio, so one multi-thread runtime serves everything.
- The choice ties routing and error mapping to axum's extractor/response model; switching frameworks later would mean rewriting both HTTP crates.
- `utoipa` annotations live near the admin routes and remain the source of truth for the generated TypeScript client.
