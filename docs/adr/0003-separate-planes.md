# ADR 0003: Separate data and control planes

- Status: Accepted
- Date: 2026-11-20
- Related: ADR 0002 (axum), ADR 0005 (React admin UI), architecture §1 and §15

## Context

The gateway serves two very different kinds of traffic: high-volume, latency-sensitive, authenticated-by-virtual-key LLM calls from projects, and low-volume CRUD/query calls from an operator using an admin session. Mixing them in one router makes auth rules harder to reason about and prevents independent exposure or scaling.

## Decision

Run two axum routers on two listeners in the same process for V1: the data plane on `:8080` (`/v1/*`) and the control plane on `:8081` (`/admin/v1/*`). They share `core` and `storage` but have separate middleware stacks and auth mechanisms. Only the admin listener serves the admin UI, `/metrics`, and is bound to localhost or a private network by default. Split into two binaries later if needed.

## Alternatives considered

- **One router with path-prefixed auth:** simpler to deploy, but a middleware mistake could expose admin endpoints on the public port.
- **Two binaries from the start:** cleaner isolation, but duplicates startup and storage wiring before there is a scaling need.

## Consequences

- The public surface has one auth model (virtual keys); the admin surface has another (HttpOnly session cookie).
- Firewall and reverse-proxy rules can expose only `:8080` to clients while keeping `:8081` private.
- Health endpoints (`/health`, `/ready`) exist on both listeners; `/metrics` only on the admin listener.
- Two listeners mean two router setups to maintain, but the shared crates keep business logic in one place.
- Splitting into two binaries later is a wiring change rather than a domain refactor.
