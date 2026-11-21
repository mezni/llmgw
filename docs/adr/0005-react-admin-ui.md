# ADR 0005: React admin UI with a generated API client

- Status: Accepted
- Date: 2026-11-20
- Related: ADR 0003 (separate planes), architecture §7 and §15, sprint plan S5–S7

## Context

The admin app needs forms for projects, keys, aliases, providers, pricing, and limits, plus a usage dashboard and a filterable request log. It must stay in sync with a Rust API that will keep changing, and it should ship in the same container without adding CORS or a second deployment unit. The UI is scaffolded in S1 (MVP) with a mock API layer, wired to the generated client in S6, and extended by each later feature sprint.

## Decision

Build a Vite + TypeScript React SPA using TanStack Router and Query, shadcn/ui with Tailwind, React Hook Form with Zod, Recharts, and TanStack Table. Generate the client from the `utoipa` OpenAPI spec (`openapi-typescript` + `openapi-fetch`) and fail CI when regenerated output drifts. Authenticate with the admin API's HttpOnly, SameSite=Strict session cookie, never storing tokens in `localStorage`. Serve the built assets from the admin listener with `tower-http`'s `ServeDir`; use a Vite dev proxy in development. Write a multi-stage Dockerfile that builds the frontend and the Rust binary into one image.

## Alternatives considered

- **Server-rendered askama + HTMX:** fewer moving parts, but a weaker fit for interactive editors, charts, and tables, and no reuse of a generated typed client.
- **SvelteKit or a separate frontend deployment:** viable, but adds a second toolchain or requires CORS and cross-origin cookie handling.
- **Hand-written TypeScript client:** faster initially, but silently drifts from the Rust API.

## Consequences

- OpenAPI generation (task 5.11) is a hard dependency for S6 and cannot be cut; CI enforces it.
- Frontend typecheck, lint, unit tests, and a Playwright smoke flow join the required checks, and Node is needed in the build stage.
- Same-origin requests need no CORS configuration, and the admin port stays off the public data plane.
- Later UI work is incremental on one app, so each feature sprint's UI task is smaller.
- Design and validation rules are documented in [conventions.md](../conventions.md).
