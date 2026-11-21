# Master Plan — Sprint Execution Workflow

One input is required: a sprint number (`S1`–`S17`). Everything else — task detail, implementation, tests, status, changelog, handoff, commit, and push — is deterministic and follows this plan.

## Source of truth

1. `docs/sprint-planning.md` — tasks, points, acceptance criteria, dependencies, cut order, gates, and decisions.
2. `docs/product.md`, `docs/architecture.md`, `docs/glossary.md` — scope, design, and naming.
3. `docs/conventions.md` — coding standards, commit/branch style, definition of done.
4. `docs/test-plan.md` — test matrix and CI gates.
5. `docs/limitations.md`, `docs/api-spec.md`, `docs/admin-api.md`, `docs/data-model.md`, `docs/security.md` — non-goals and contracts.
6. `CHANGELOG.md` — release history.
7. `STATUS.md`, `HANDOFF.md` — prior progress.

If two sources conflict, the newest explicit decision in `docs/sprint-planning.md` §7 or an ADR wins. If that still does not resolve it, ask before proceeding.

Version mapping is fixed: S1–S3 → 0.1.0 (MVP), S4–S7 → 0.2.0 (V1), S8–S11 → 0.3.0 (V2), S12–S17 → 0.4.0 (V3). There is no 1.0.0 milestone.

## Artifacts

| File | Purpose | Lifetime |
|------|---------|----------|
| `STATUS.md` | Durable per-sprint status, task checklist, test evidence, decisions, blockers, sprint history | Accumulates; one section per sprint |
| `HANDOFF.md` | Session handoff for the next agent/session | Overwritten every session |
| `CHANGELOG.md` | User-visible changes, Keep a Changelog format | Permanent |
| Git branch + commits | One branch per sprint, small Conventional Commits | Permanent |

## Phase 0 — Preflight

- [ ] Confirm the sprint exists in `docs/sprint-planning.md` and note its version and goal.
- [ ] Check `git status`. Work from a clean tree; if dirty, stop and ask rather than stashing someone else's work.
- [ ] Read `STATUS.md` and `HANDOFF.md`. If the requested sprint is already in progress, resume from the first unchecked task; do not redo completed work.
- [ ] Check entry criteria and dependencies (`docs/sprint-planning.md` §2 and §3). Blocked entry is a stop condition.
- [ ] Check capacity: commit about 16 points; list the rest as stretch/carryover using the cut order in §5. Never cut a "Never cut" item.
- [ ] Create branch `feat/s<n>-<short-slug>` from `main` (lowercase, hyphens).

## Phase 1 — Detail the tasks

Expand the sprint table into an ordered task list. For each task record:

| Field | Content |
|-------|---------|
| Task id | e.g. `4.6` (keep the sprint-planning id) |
| Description | One sentence from the sprint table |
| Acceptance criteria | Verbatim, plus any testable clarification |
| Depends on | Task ids or prior sprints |
| Files | Expected source, test, and doc files |
| Tests | Unit/adapter/integration/UI/E2E cases that prove the criteria |
| Docs | Docs that must change in the same commit |
| Points | From the sprint table |

Order by dependency. Write the plan into `STATUS.md` before coding, then proceed without waiting unless acceptance criteria are ambiguous.

## Phase 2 — Implement

- Follow `docs/conventions.md` for layout, naming, errors, async code, and React state.
- A bug fix gets a failing test first; new behavior gets a test in the same change.
- One logical change per commit. Do not push until Phase 7.
- Update affected docs in the same change: API changes touch `api-spec.md`/`admin-api.md`, schema changes touch `data-model.md`, behavior changes touch `provider-compatibility.md`/`limitations.md`.
- If the control plane changes, regenerate both artifacts before finishing:
  ```bash
  cargo run -p admin-api --bin export-openapi > docs/openapi.json
  npx openapi-typescript docs/openapi.json -o admin-ui/src/api/schema.d.ts
  ```
- Never commit secrets, `.env`, credentials, or real prompt fixtures. Comment the *why*, never the *what*.

## Phase 3 — Test

Run every applicable check and record the command plus result in `STATUS.md`. A failure blocks commit and push: fix it, or stop and hand off.

| Check | Command | When |
|-------|---------|------|
| Format | `cargo fmt --all -- --check` | Every sprint with Rust changes |
| Lint | `cargo clippy --all-targets -- -D warnings` | Every sprint with Rust changes |
| Rust tests | `cargo test` | Every sprint with Rust changes |
| Frontend | `npm run typecheck && npm run lint && npm test` | Every sprint with UI changes |
| OpenAPI drift | `git diff --exit-code -- docs/openapi.json admin-ui/src/api/schema.d.ts` | After any control-plane change |
| Migration | Fresh `sqlx migrate run` + schema snapshot | Any migration change |
| E2E | `npm run e2e` | S6 onward |
| SDK contract | OpenAI Python/TS suites; Anthropic SDK in S12+ | S7 onward |
| Load/chaos | `oha`/`k6`, disconnect storms | S7, S17, manual releases |
| Security | `cargo audit`, `cargo deny`, secret scan | S17 and release gates |

## Phase 4 — Write `STATUS.md`

Use this structure; keep it factual and append-only across sprints:

```markdown
# Status

## Current sprint: S__ — <goal> (<version>)
- Branch: <branch>
- Started / updated: <date>
- State: in progress | blocked | complete

### Task plan
| Id | Task | Pts | State | Evidence |
|----|------|-----|-------|----------|
| 1.1 | ... | 2 | done | commit <sha>, tests pass |

### Verification
| Check | Command | Result |
|-------|---------|--------|
| ... | ... | pass/fail (details) |

### Decisions and deviations
- ...

### Deferred / carryover
- <task id> — <reason, cut order reference>

### Blockers
- None | <blocker and what is needed>

### Definition of done
- [ ] Acceptance criteria met
- [ ] fmt, clippy, tests, typecheck/lint/UI tests pass
- [ ] Mock-provider scenario covered; real provider where relevant
- [ ] Docs and CHANGELOG updated
- [ ] No secrets in logs, fixtures, or repo
- [ ] Generated client regenerated if OpenAPI changed
- [ ] Committed on a short-lived branch and pushed

## Sprint history
| Sprint | Version | State | Commit / tag | Notes |
|--------|---------|-------|--------------|-------|
```

## Phase 5 — Update `CHANGELOG.md`

- Follow Keep a Changelog and the existing `[Unreleased] — targeting 0.x.0` heading.
- Use only the standard sections: `Added`, `Changed`, `Fixed`, `Removed`, `Deprecated`, `Security`.
- One bullet per user-visible change; mention migrations, upgrade steps, and breaking changes explicitly.
- At a version checkpoint (end of S3, S7, S11, S17), convert `[Unreleased]` into `## [0.x.0] - YYYY-MM-DD`, add release notes, and start a new `[Unreleased]` for the next version.
- Never invent a 1.0.0 milestone.

## Phase 6 — Write `HANDOFF.md`

Overwrite this file every session:

```markdown
# Session handoff

- Date:
- Sprint / version:
- Branch:
- HEAD commit:
- Push status: pushed | local only (reason)
- State: complete | blocked | paused

## Completed this session
- ...

## Test evidence
- <command>: pass/fail

## Files changed
- ...

## Deferred / carryover
- <task id> — <reason>

## Open decisions and risks
- ...

## Gotchas for the next session
- ...

## Next step
Paste MASTER_PROMPT.md with: Sprint = S__
```

## Phase 7 — Commit and push

- Final `git status` and `git diff --check`; confirm no secrets or unintended files.
- Stage only intended files.
- Commit with `<type>(<scope>): <imperative summary>` using the types/scopes in `docs/conventions.md`. Include the sprint id and a short test summary in the body.
- Push: `git push -u origin <branch>`.
- Do not force push, amend, skip hooks, or create empty commits. If push fails, keep the commit local, record it in `HANDOFF.md`, and report.
- Final report: sprint, tasks done/deferred, test evidence, commit SHA, branch, and paths to `STATUS.md` and `HANDOFF.md`.

## Stop conditions

Stop, write both status files with the blocker, and ask before continuing when:

- any lint, typecheck, or test gate fails and cannot be fixed in scope;
- acceptance criteria are ambiguous or require inventing scope;
- work conflicts with `docs/limitations.md` or a "Never cut" item;
- secrets or credentials appear in files, logs, or fixtures;
- `git status` is dirty at preflight, or push/authentication fails.

## Definition of done

A sprint is done only when all acceptance criteria for the committed tasks pass, all applicable gates are green, the mock-provider scenarios (and real-provider checks where relevant) are covered, docs and `CHANGELOG.md` are updated, no secrets are present, the generated client is current if OpenAPI changed, and the branch is committed and pushed with `STATUS.md` and `HANDOFF.md` written.
