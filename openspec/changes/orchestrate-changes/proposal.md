# Proposal

## Why

Changes are filed ahead of time with `/create-change` and run one at a time with `/proceed-change`. With several changes in the backlog, picking which one can start, which must wait for another, and which can run side by side is done by hand. An orchestrator that knows the dependencies between changes and runs each in its own session removes that manual step.

## What Changes

- Declare dependencies between OpenSpec changes, kept apart from the OpenSpec files (OpenSpec drops unknown keys in `.openspec.yaml`).
- Add an orchestrator that reads the backlog and its dependencies, and runs ready changes through `/proceed-change`:
  - independent changes in parallel, each in its own session and worktree;
  - dependent changes in sequence, after the changes they depend on are on `main`.
- *How* (dependency format, launcher, limits on parallel E2E and version bumps) is decided in `design.md`, after `/create-change` and `/proceed-change` are proven on real changes.

## Capabilities

### New Capabilities

None. This is development tooling, not app behavior: `skip_specs: true`.

### Modified Capabilities

None.

## Scope

- Current stage: PoC. Target stage: PoC (tooling only).
- Starts only after `/create-change` and `/proceed-change` have run end to end on at least one real change each.
- Acceptance bar: two independent changes run in parallel and one dependent change runs after its dependency merged, without manual scheduling.

### Out of scope

- Changes to `app/` or `e2e-test/`.
- Choosing the dependency format and the launcher (decided in `design.md`).

## Version

No release: no app behavior changes.

## Impact

- agent-kit `claude-code/skills/` (new orchestrator skill, maybe changes to `/proceed-change`; both live in agent-kit per `kit-placement.md`), `AGENTS.md` § Workflow, possibly `justfile` (a planner recipe), a dependency file per change or per backlog.
- Shared E2E ports 8080/8081 and the version bump in `app/Cargo.toml` limit what can run in parallel.
