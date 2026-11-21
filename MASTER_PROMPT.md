# Master Prompt — Run a Sprint

Copy the block below, replace `S__`, and send it. Nothing else is needed.

````markdown
Execute sprint S__ for the LLM Gateway. Follow MASTER_PLAN.md end to end.

1. Preflight: read docs/sprint-planning.md plus the product, architecture, conventions, test-plan, and limitations docs, CHANGELOG.md, STATUS.md, and HANDOFF.md. Check git state, verify entry criteria and dependencies, and create branch feat/s<number>-<short-slug> from main. If STATUS.md shows this sprint already in progress, resume from the first unchecked task instead of redoing work.
2. Detail the tasks: expand the sprint table into an ordered list with task ids, acceptance criteria, dependencies, affected files, tests, doc updates, and points. Commit about 16 points; defer the rest using the cut order in sprint-planning.md §5, and never cut a "Never cut" item. Write this plan into STATUS.md before coding.
3. Implement: follow docs/conventions.md, write tests with new behavior, update affected docs in the same change, and regenerate docs/openapi.json and admin-ui/src/api/schema.d.ts if the control plane changes. Make small Conventional Commits; do not push yet.
4. Test: run cargo fmt --all -- --check, cargo clippy --all-targets -- -D warnings, cargo test, npm run typecheck && npm run lint && npm test, the OpenAPI drift check, and the migration, E2E, SDK contract, and security checks applicable to this sprint. Record every command and result in STATUS.md. If anything fails, fix it or stop and report — never push red.
5. Write STATUS.md using the template in MASTER_PLAN.md.
6. Update CHANGELOG.md under [Unreleased] in Keep a Changelog format. Roll the version only at S3/S7/S11/S17 (0.1.0/0.2.0/0.3.0/0.4.0).
7. Write HANDOFF.md using the template in MASTER_PLAN.md: branch, HEAD commit, push status, completed tasks, test evidence, files changed, deferred tasks, open risks, gotchas, and the exact next command.
8. Stage only intended files, verify no secrets, commit with a Conventional Commit message, and push with git push -u origin <branch>. Do not force push, amend, skip hooks, or make empty commits.
9. Final report: sprint, tasks completed and deferred, test evidence, commit SHA, branch, and paths to STATUS.md and HANDOFF.md.

Rules: docs/sprint-planning.md is the source of truth. Do not invent scope, versions, or a 1.0.0 milestone. Ask me before changing acceptance criteria, cutting a "Never cut" item, or working around a conflict with docs/limitations.md. Stop and hand off if blocked.
````

## Optional overrides

Append any of these lines to the prompt when needed:

- `Resume only: start from HANDOFF.md; do not redo completed tasks.`
- `Sprint constraint: implement only tasks <ids>; defer the rest with reasons.`
- `Release checkpoint: roll <version> in CHANGELOG.md and prepare the tag (do not push tags).`
- `Status only: skip implementation; refresh STATUS.md and HANDOFF.md from the current tree.`
