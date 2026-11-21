# Contributing

Thanks for helping build the gateway. The full coding standards are in [docs/conventions.md](docs/conventions.md); this file is the short workflow.

## Before you start

- Read the [product document](docs/product.md), [architecture](docs/architecture.md), and the [glossary](docs/glossary.md) so names match across docs and code.
- Check [limitations.md](docs/limitations.md) before implementing something that may already be a deliberate non-goal.
- Pick work from [sprint-planning.md](docs/sprint-planning.md). Keep changes small enough to review.

## Setup

```bash
cp .env.example .env          # once the template lands
export DATABASE_URL=sqlite://./gateway.db
export GATEWAY_MASTER_KEY=<base64 32-byte key>
export ADMIN_BOOTSTRAP_EMAIL=admin@example.com
export ADMIN_BOOTSTRAP_PASSWORD=<strong password>
export RUST_LOG=info
cargo build
npm install
```

All settings and defaults are in [docs/config-reference.md](docs/config-reference.md). Never commit `.env`, keys, credentials, or real prompt fixtures.

## Workflow

1. Create a branch from `main`: `feat/<short-name>`, `fix/<short-name>`, or `docs/<short-name>`.
2. Make the change with tests. Use Conventional Commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
3. Update the docs in the same change: API changes touch [api-spec.md](docs/api-spec.md) or [admin-api.md](docs/admin-api.md); schema changes touch [data-model.md](docs/data-model.md); behavior changes touch [provider-compatibility.md](docs/provider-compatibility.md) or [limitations.md](docs/limitations.md).
4. Add an entry under `[Unreleased]` in [CHANGELOG.md](CHANGELOG.md). Versions increment 0.1.0 → 0.2.0 → 0.3.0 → 0.4.0; do not invent a 1.0.0 milestone.
5. Open a pull request with a short description, the test evidence, and any migration or upgrade notes.

## Checks

Run these before pushing:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
npm run typecheck && npm run lint && npm test
```

On changes to the control plane, also regenerate and commit both artifacts:

```bash
cargo run -p admin-api --bin export-openapi > docs/openapi.json
npx openapi-typescript docs/openapi.json -o admin-ui/src/api/schema.d.ts
```

CI repeats these checks, runs the SDK contract suite and Playwright smoke flows, audits dependencies, and fails on OpenAPI drift. See [docs/test-plan.md](docs/test-plan.md) for the full matrix.

## Definition of done

A change is done when it has tests for the happy path, a validation failure, and a provider failure; its log/audit side effects are covered; docs and changelog are updated; and it can be deployed without a manual database edit. Streaming changes additionally need cut-stream and malformed-SSE cases. See [docs/conventions.md](docs/conventions.md#10-definition-of-done) for the complete checklist.
