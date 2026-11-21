# ADR 0004: Clients request aliases, not model names

- Status: Accepted
- Date: 2026-11-20
- Related: architecture §4.2 and §6, product §3 and §7.2, sprint plan S4

## Context

Applications hardcode provider model names (for example `claude-…` or `gpt-…`). Swapping a model then requires a code change and redeploy in every project. The gateway also needs a place to attach fallback order, timeouts, retry limits, and pricing, none of which belong in application code.

## Decision

Expose stable aliases such as `fast` and `smart` as the client-facing model field. Resolve each alias per request from an in-memory `ConfigSnapshot` to a primary target, an ordered fallback chain, a timeout, and an attempt limit. Store aliases, targets, pricing, and per-key alias allowlists in the database; reload the snapshot atomically on admin mutations.

## Alternatives considered

- **Pass real model names through:** simplest, but every model change becomes a client redeploy and pricing/fallback policy has nowhere to live.
- **Client-side configuration files:** scatters routing decisions across repos and cannot be changed centrally.
- **Model names with a default env override:** one global switch, no per-project allowlist, no fallbacks.

## Consequences

- Switching a model is a database edit through the admin API; no project redeploys.
- The request path gains one lock-free snapshot lookup, which is acceptable and keeps DB traffic off the hot path.
- Unknown aliases return 404 and aliases not permitted for a key return 403, so mistakes are explicit.
- The allowlist is per alias, not per upstream model; clients never see provider-specific names.
- V3 routing policies (weights, rules, canaries) build on the same alias abstraction.
