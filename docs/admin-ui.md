# Admin UI

Pages, routes, component structure, state, auth flow, and design basics for the React admin app. The stack decision is in [ADR 0005](adr/0005-react-admin-ui.md), the contract it consumes is [admin-api.md](admin-api.md), and the delivery sequence is in [sprint-planning.md](sprint-planning.md).

## 1. Scope and stack

- Vite + TypeScript SPA scaffolded in S1 (MVP) with `react-router-dom`, local CSS primitives, and a local mock API; S6 replaces the mock with the generated client from [openapi.json](openapi.json) and connects real endpoints.
- Planned S6 stack: TanStack Router for typed routes, TanStack Query for server state, shadcn/ui + Tailwind for components, React Hook Form + Zod for forms, Recharts for dashboards, TanStack Table for logs.
- Tests: Vitest + React Testing Library for components against the local mock API; Playwright smoke flows arrive in S6.
- One container: the built files are served by the Rust admin listener on `:8081`; no CORS, no second deployment unit. The scaffold makes no HTTP calls; S6 adds the `vite dev` proxy to `:8081`.

## 2. Routes and pages

| Route | Page | Purpose |
|-------|------|---------|
| `/login` | `LoginPage` | Email/password form; redirects to `/` when already authenticated |
| `/` | `DashboardPage` | Requests, tokens, cost, latency, error rate over time |
| `/projects` | `ProjectsPage` | Project list with monthly spend and budget |
| `/projects/:projectId` | `ProjectDetailPage` | Edit budget; manage that project's keys |
| `/keys` | `KeysPage` | All key prefixes, status, limits, revoke |
| `/aliases` | `AliasesPage` | Alias list and ordered fallback-chain editor |
| `/providers` | `ProvidersPage` | Provider list, credentials form, test-connection button |
| `/models` | `ModelsPage` | Upstream model names and pricing |
| `/limits` | `LimitsPage` | Per-key RPM/TPM and project budgets |
| `/requests` | `RequestsPage` | Filterable log table with request detail drawer |
| `/playground` | `PlaygroundPage` | Send a chat request through an alias and watch the SSE stream |
| `/audit` | `AuditPage` | Admin action history |
| `*` | `NotFoundPage` | Unknown route |

Route definitions live in `admin-ui/src/routes/`. Each route file exports its loader/query keys and lazily imports the page component. The root layout (`AppShell`) is the only place that owns global navigation and the auth guard.

## 3. Component structure

```text
admin-ui/src/
├── main.tsx                 # browser bootstrap
├── App.tsx                  # route table
├── api/
│   ├── client.ts            # AdminApi contract and local api instance
│   └── mock.ts              # in-memory demo data
├── components/
│   ├── common/              # StatusBadge
│   └── layout/              # AppShell
├── lib/format.ts            # number, currency, and percent formatting
├── pages/                   # Dashboard, Projects, Keys, NotFound
├── styles.css               # semantic tokens and layout primitives
└── test/setup.ts            # Vitest DOM matchers
```

Domain components are pure and receive data through props or hooks. The planned S6 layout adds `routes/`, `features/`, domain component folders, generated `components/ui/*`, and generated `api/schema.d.ts`; generated files are never edited directly.

## 4. State management

- **Server state:** the MVP scaffold uses local `useState` with async calls to `AdminApi`; TanStack Query and structured query keys arrive in S6.
- **URL state:** the scaffold uses route paths only; shareable filters and pagination arrive with TanStack Router in S6.
- **Form state:** the scaffold has no forms; React Hook Form and Zod arrive in S6.
- **Session state:** the scaffold has no login or credentials; the documented memory-only session flow arrives in S6.
- **Ephemeral UI state:** local `useState` for loaded mock data and retry state. No Redux, no global mutable store.

A created key's secret is held only in the component state of `SecretReveal` for the lifetime of that dialog. Closing or navigating away discards it; a later list fetch cannot bring it back.

## 5. Auth flow

The MVP scaffold has no auth flow, cookies, or credentials. S6 implements the V1 flow below:

1. Unauthenticated visit to any route renders `LoginPage`; a successful `POST /admin/v1/auth/login` sets an `HttpOnly`, `SameSite=Strict` cookie and redirects to `/`.
2. `AppShell` checks session state on mount. A `401` from any query triggers one silent re-fetch, then a redirect to `/login?next=<route>`; no error toast for the expected 401.
3. Protected routes render a loading shell until the session query resolves, avoiding a flash of login on refresh.
4. `logout` posts to `/admin/v1/auth/logout`, clears the in-memory session, and navigates to `/login`.
5. CSRF is mitigated by same-origin requests, JSON-only bodies, and `SameSite=Strict`; V1 needs no token. If a future deployment relaxes `SameSite`, add a double-submit token before changing the cookie policy.
6. Roles are a V2 feature; the UI already checks a `role` field if present but does not depend on it in V1.

## 6. Design basics

- Neutral slate surfaces with one accent color for primary actions; semantic green/amber/red for success, warning, and error. Status colors are never the only signal—labels or icons accompany them.
- 8px spacing scale, 4px radius for controls and 8px for cards, one subtle border color, minimal shadows. Prefer density over decoration: this is an operator tool.
- Typography: system sans stack, 14px body, 12px metadata, semibold section headings. Numeric values use tabular figures.
- Responsive from 1024px up; below that the sidebar collapses to icons. Tables scroll horizontally rather than hiding columns.
- Every input has a visible label, every icon-only button has an accessible name, focus rings are never removed, and dialogs trap focus and close on Escape.
- Loading uses skeleton rows in tables and charts; empty states explain how to create the first item; errors show a retry button and the safe `message` from the API.
- Charts default to a 30-day window with day buckets; tooltips show exact values and UTC timestamps. Money is formatted to cents with a currency symbol; tokens use thousands separators.
- Destructive actions (delete project, revoke key, purge cache) require a `ConfirmDialog` that names the target.

## 7. Testing and delivery

- Current component tests cover the dashboard, key prefixes, and the not-found route with the local mock API.
- S6 component tests cover form validation, secret reveal-once behavior, and error/empty states with an MSW-backed generated client.
- Playwright smoke flows in S6: login → create project/key → dashboard shows usage; plus revoking a key and seeing it disappear from the list.
- CI runs `npm run typecheck && npm run lint && npm test`, regenerates `schema.d.ts` from `openapi.json` after a control-plane change, and fails on drift. `npm run e2e` runs against the built app served by the admin listener in S6.
- See [test-plan.md](test-plan.md) for the full testing strategy and [observability.md](observability.md) for the fields the UI charts consume.
