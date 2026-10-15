# Proposal

## Why

Changes are filed ahead of time with `/create-change` and run one at a time with `/proceed-change`. With several changes in the backlog, picking which one can start, which must wait for another, and which can run side by side is done by hand. An orchestrator that knows the dependencies between changes and runs each in its own session removes that manual step.

## What Changes

The work is split in two OpenSpec changes, per `kit-placement.md`:

- **agent-kit change `orchestrate-changes`** (generic, reusable in every kit project):
  - an `orchestration.yaml` file next to a change's OpenSpec documents, with `depends_on` (names of other changes) and `autonomy` (`auto`, `review`, `explore`); OpenSpec drops unknown keys in `.openspec.yaml`, so the file lives beside it;
  - `kit backlog` reads it and tells, per change, whether it is ready, blocked by a named change not yet on `main`, waiting for the user (`explore`) or already running (branch or open PR);
  - a new `/orchestrate` skill that starts every ready change in its own linked desktop session running `/proceed-change <name>`, each in its own worktree, and starts a blocked change once the changes it depends on are on `main`;
  - `/create-change` asks for dependencies and autonomy and writes the file; `/proceed-change` takes its start answers from the orchestrator instead of asking again.
- **this change** (voicepuncher project values): `AGENTS.md` § Workflow names `/orchestrate`, the `orchestration.yaml` file and the limits on parallel work in this project (shared E2E ports serialized by the machine-wide test lock; parallel version bumps resolved when the branch merges `origin/main`).

Decisions taken before design (user, 2026-10-15): no further exploration; launcher is linked desktop sessions (the six-change run of 2026-10-15 used them); dependencies live in one file per change folder, archived with the change.

## Capabilities

### New Capabilities

None in voicepuncher. This is development tooling, not app behavior: `skip_specs: true`. The agent-kit change carries its own spec.

### Modified Capabilities

None.

## Scope

- Current stage: PoC. Target stage: PoC (tooling only).
- Depends on the agent-kit change `orchestrate-changes` being on agent-kit `main`.
- Acceptance bar: `kit backlog` in this repo shows two independent changes as ready and a change whose `orchestration.yaml` names an unmerged change as blocked; `/orchestrate` starts the ready ones in parallel sessions and the blocked one after its dependency merged, without manual scheduling.

### Out of scope

- Changes to `app/`, `command-spotting/`, `e2e-test/`, `justfile`.
- Terminal tabs or cloud sessions as launchers.
- A board outside the chat (a mod or a view of its own).

## Version

No release: no app behavior changes.

## Impact

- `AGENTS.md` § Workflow.
- agent-kit: `claude-code/skills/{orchestrate,create-change,proceed-change}`, `claude-code/kit/internal/backlog` (separate change and PR in agent-kit).
