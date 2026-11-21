# Conventions

Coding, testing, and collaboration rules for the LLM Gateway. These follow the design in [architecture.md](architecture.md) and the working agreement in [sprint-planning.md](sprint-planning.md). Tooling output (`rustfmt`, `clippy`, ESLint/TypeScript, Prettier if adopted) is the final authority on formatting; when in doubt, match the surrounding code.

## 1. Repository and module layout

### Rust workspace

```
crates/
├── core/        ChatRequest, ChatResponse, Usage, errors — no I/O, no async runtime dependency
├── providers/   Provider trait + adapters — talks to upstreams; knows nothing about projects, keys, or the DB
├── storage/     sqlx models, repositories, migrations — knows nothing about HTTP
├── gateway/     data plane: routes, tower middleware, routing engine, execution engine
├── admin-api/   control plane: routes, admin auth, OpenAPI types
└── server/      binary: config, wiring, startup, shutdown
```

Rules:

- Dependencies point one way: `server → gateway/admin-api → storage → core` and `gateway → providers → core`. Never introduce a cycle or reach into a higher layer from a lower one.
- `core` stays pure: no `reqwest`, no `sqlx`, no `tokio::net`, no env access.
- HTTP types (axum extractors, status codes) stay in `gateway` and `admin-api`; convert to and from `core` types at the boundary.
- Prefer plain `foo.rs` modules with a `foo/` directory for children over `mod.rs` files. Re-export the public surface from `lib.rs`; keep internals private.
- One concern per module. If a file exceeds roughly 300 lines or mixes unrelated responsibilities, split it.
- Put SQL in `storage`, never inline in handlers. Migrations are append-only once shipped.

### React app

```
admin-ui/src/
├── main.tsx, App.tsx        app bootstrap and router
├── routes/                  route components (thin; delegate to features)
├── features/<name>/         components, hooks, and tests for one feature
├── features/api/            query-key factories and API error parsing
├── features/auth/           session helpers
├── components/ui/           shadcn/ui primitives — generated, do not hand-edit
├── api/                     generated client from the OpenAPI spec — never edit manually
├── lib/                     formatters, date helpers, error parsing
└── test/                    test setup and fixtures
```

Rules:

- Organize by feature, not by file type. A feature owns its components, hooks, schemas, and tests.
- The API client is generated (`openapi-typescript` + `openapi-fetch`). CI regenerates it and fails on drift; never patch generated files by hand.
- Route components only read route params and render feature components. No data fetching or business rules in routes.
- Keep shared primitives in `components/ui/`; add app-specific composition in `features/<name>/`.

## 2. Naming

| Kind | Rust | TypeScript / React |
|------|------|--------------------|
| Types, traits, components | `ChatRequest`, `Provider`, `ProjectCard` (PascalCase) | `ProjectCard`, `UsageChart` (PascalCase, matching filename) |
| Functions, variables, hooks | `resolve_alias`, `project_id` (snake_case) | `resolveAlias`, `projectId` (camelCase) |
| Modules / files / directories | `alias_targets.rs` (snake_case) | `project-card.tsx`, `use-projects.ts` (kebab-case files) |
| Constants | `MAX_RETRIES` (SCREAMING_SNAKE_CASE) | `DEFAULT_PAGE_SIZE` for module constants |
| Booleans | `is_retryable`, `has_fallback` | `isLoading`, `hasFallback` |
| Collections | `targets`, `aliases` (plural) | `targets`, `aliases` (plural) |
| Error enums | `ProviderError`, `StorageError` | `ApiError`, `ValidationError` (no `I` prefix on interfaces) |
| Tests | `test_retry_honors_retry_after` | `it('rejects a revoked key', ...)` |

Other rules:

- HTTP routes use plural resource names: `/admin/v1/projects`, `/aliases`. Gateway headers use `X-Gateway-*` (for example `X-Gateway-Tag`).
- Database tables and columns are snake_case and plural where appropriate: `api_keys`, `key_allowed_aliases`, `input_tokens`.
- Do not abbreviate domain words (`req`, `res`, `cfg`) outside of local variables with obvious scope. `id`, `ttft`, and `usage` are accepted domain terms.
- Name tests after the behavior, not the implementation.

## 3. Rust coding standards

- Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` before proposing a change. CI enforces both.
- Borrow by default; clone at boundaries where ownership is actually needed (`Arc<dyn Provider>`, `ArcSwap<ConfigSnapshot>`). Do not clone to silence the borrow checker without understanding the lifetime.
- Use `thiserror` for domain error enums in libraries. Reserve `anyhow` for the `server` binary's startup path; never return `anyhow::Error` from `core`, `providers`, `storage`, `gateway`, or `admin-api` public APIs.
- No `unwrap`, `expect`, or `panic!` in request-handling code. `expect` is acceptable in `main` for configuration that must be valid to start, and in tests.
- No blocking work on the async runtime: use `spawn_blocking` for CPU-heavy or blocking calls. Keep `reqwest` clients shared per provider.
- Treat streams as cancel-safe: dropping the response body cancels the upstream request; cleanup belongs in the tap's `Drop` implementation.
- Keep hot paths allocation-light and lock-free where practical: config reads go through `ArcSwap`, not the DB; per-key state uses `DashMap`.
- Prefer `Arc<dyn Provider>` for the registry and generics for internal helpers. Keep `extra` passthrough typed as `serde_json::Map` at the edge only.
- Feature flags (for example a future Postgres backend) live in `storage`; no other crate learns about the concrete database.

## 4. Error handling rules

### Rust

- Every layer converts errors at its boundary: `ProviderError` → route error → OpenAI-shaped JSON. Never pass a raw upstream body or `sqlx` error to the client.
- `ProviderError` drives routing, so its classification is part of the contract:

| Variant | Retryable | Handling |
|---------|-----------|----------|
| `RateLimited { retry_after }` | Yes | Backoff or honor `Retry-After`, then next attempt |
| `Upstream5xx { status }` | Yes | Backoff, then next attempt |
| `Timeout` | Yes | Backoff, then next attempt |
| `Network(_)` | Yes | Backoff, then next attempt |
| `BadRequest { status, body }` | No | Return to client immediately |
| `Auth` | No | Return error and alert the operator |
| `Stream(_)` | No retry | Emit a final SSE error event; record in the log |

- Retry only before the first streamed byte. After the stream commits, surface the failure and never restart it.
- Attach context where an error crosses a layer, but log once at the outermost boundary. Include `request_id`, project, and alias; never include provider credentials, full prompts, or `Authorization`.
- Usage logging is best-effort: `try_send` failures increment a metric and do not fail the request. Budget enforcement must not depend on this channel — spend is accounted on the request path before enqueueing.
- Client-facing errors follow the OpenAI shape (`{"error":{"message":...,"type":...,"code":...}}`) and the status mapping in [architecture §10](architecture.md).

### React

- Treat any non-2xx response as an error; centralize extraction of `error.message`, `type`, and `code` in `api/client.ts`.
- Show a readable message and a retry action where the operation is safe. Never render raw stack traces or upstream bodies.
- On 401, redirect to login. On 403, explain that the key or alias is not permitted. On 429, show `Retry-After` when present.
- Form errors come from Zod validation; server-side rejections are mapped back to fields when possible.
- Never swallow errors silently. If a failure cannot be surfaced in the UI, report it through the app's error boundary or logger.

## 5. React coding standards

- TypeScript `strict` mode; no `any`. Use `unknown` plus narrowing for untrusted data.
- Functional components and hooks only. All server state lives in TanStack Query — no `useEffect` fetch loops, no hand-rolled cache in component state.
- After every mutation, invalidate the affected query keys through the shared factories in `features/api`.
- Define forms with React Hook Form and Zod; reuse schemas between client and server-side validation where practical.
- Keep business rules out of JSX: extract them into hooks or pure functions in the feature folder.
- Use shadcn/ui primitives and Tailwind utilities; avoid ad-hoc CSS files. Keep colors and spacing on design tokens.
- Accessibility is not optional: label inputs, keep keyboard focus visible, use semantic elements, and give icon buttons accessible names.
- Charts use Recharts, tables use TanStack Table. Prefer functional correctness over visual polish during feature sprints.
- Session auth uses the HttpOnly, SameSite=Strict cookie. No tokens in `localStorage`; API calls include credentials.

## 6. Testing rules

| Level | Tool | Expectation |
|-------|------|-------------|
| Unit | `cargo test` | Pure logic: cost calculation, backoff, rate limiter, alias resolution, type conversions |
| Adapter | `cargo test` + fixtures | Recorded OpenAI/Anthropic payloads, including fragmented and malformed SSE |
| Integration | axum mock provider | Success, slow, 429 with `Retry-After`, 500, mid-stream cut, malformed SSE |
| Admin | in-memory SQLite | CRUD, auth, audit trail, config reload |
| Frontend unit | Vitest + React Testing Library | Behavior users can observe, not component internals |
| E2E | Playwright | Login → create project/key → dashboard; plus each sprint's demo flow |
| Contract / load | OpenAI and Anthropic SDKs, `oha`/`k6` | Later milestones; see [architecture §13](architecture.md) |

Rules:

- A bug fix needs a failing test first. New behavior needs a test in the same change.
- Tests must be deterministic and offline by default. Use the mock provider, not a real upstream; real-provider runs are deliberate and use small prompts.
- Never sleep to synchronize a test. Wait on conditions or inject clocks/timers.
- Critical paths require coverage: auth, budget enforcement, retry/fallback classification, token accounting, stream cancellation, and config reload.
- Do not skip or mark a flaky test as ignored to make CI pass. Quarantine it with an owner and fix it.
- E2E tests cover one critical flow each and keep assertions on user-visible outcomes.
- CI runs formatting, lints, all test tiers, typecheck, and the frontend unit suite; Playwright runs against the built app on `:8081`.

## 7. Commit and branch style

Branches are short-lived and cut from `main`:

```
feat/alias-fallback-editor
fix/retry-after-429
docs/merge-sprint-plan
test/stream-cancellation
```

- Use `<type>/<short-slug>`; lowercase, hyphens, no ticket IDs unless one exists.
- One logical change per branch. Merge back to `main` and delete the branch — even when working solo.

Commits follow Conventional Commits:

```
<type>(<scope>): <imperative summary>

[optional body]

[optional BREAKING CHANGE: ...]
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `build`, `ci`, `perf`, `spec`. Scopes: `core`, `providers`, `storage`, `gateway`, `admin-api`, `server`, `admin-ui`, `docs`.

Examples:

```
feat(gateway): honor Retry-After on upstream 429
fix(admin-ui): invalidate alias queries after reorder
docs: merge V2/V3 sprint plan
```

- Write the summary in the imperative mood, lowercase, without a trailing period.
- Keep each commit buildable and green. Do not commit generated files unless the change intentionally updates them.
- Never commit secrets, `.env` files, credentials, or full prompt/response fixtures containing real data.

## 8. Definition of done

A task is done only when all of the following hold:

- The task's acceptance criteria from the sprint plan are met.
- Code compiles; `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` pass.
- Tests for the new behavior are written and passing; frontend typecheck, lint, and unit tests pass where applicable.
- The scenario works against the mock provider, and against a real provider where relevant.
- UI changes have a passing Playwright flow for the affected demo path.
- No secrets appear in logs, fixtures, or the repository; full prompt/response logging remains opt-in.
- Documentation and configuration references are updated when behavior or config changed.
- The generated API client is regenerated and the CI drift check passes if the OpenAPI spec changed.
- Merged to `main` through a short-lived branch.
