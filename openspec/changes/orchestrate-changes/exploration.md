# Exploration notes

Working notes from the first exploration (2026-10-05). Not a design: the use case is not settled yet. `design.md` is written once the open questions below are answered.

## User journey

1. The user files several changes with `/create-change`, each with a description, optionally its dependencies and how much of the user it needs.
2. The user starts the orchestrator (working name `/orchestrate`). It builds the dependency graph and starts every ready change in **its own chat session** running `/proceed-change <name>`: independent changes in parallel, each in its own worktree; a dependent change waits until the changes it depends on are on `main`.
3. The orchestrator shows a board: running / **waiting for the user** / blocked / done. When an agent needs the user, a push notification arrives; the user answers in that agent's chat, and the agent and the orchestrator go on.

## Launcher options

| Option | User can watch and answer | Orchestrator learns a session finished | Problem |
|---|---|---|---|
| Linked desktop-app sessions (`start_session`) | yes: own chat in the sidebar, split view | yes: the parent is told when a child's turn ends | the tool was not exposed in the exploring session; find out how to enable it |
| Terminal tabs: `claude "/proceed-change X"` in a worktree | yes: an interactive `claude` | no: polling git/gh only | polling |
| Cloud sessions | yes | yes | not usable: E2E needs macOS, Metal and the local models |
| Subagents (`Agent` tool) | no | yes | cannot ask the user |

Leaning: linked desktop sessions, terminal tabs as fallback. The orchestrator keeps no state of its own: it derives each change's stage from facts, like `proceed-change` § Resume (branch, PR, merge, tag).

## Limits on parallel work (this project)

- **E2E ports 8080/8081** are shared across checkouts: E2E cannot run in two worktrees at once. A shared lock around `just check` / `just full-check` lets coding run in parallel while E2E runs one at a time.
- **Version bump in `app/Cargo.toml`**: two PATCH bumps always conflict. Options: bump at merge time instead of in `apply`, or never run two version-bumping changes at once.
- **Implicit dependencies**: two changes touching the same code depend on each other even when nothing says so. Example: `cut-chunks-at-pauses` and `keep-words-after-pauses` both concern chunk seams and the `e2e-seams` numbers. Declare `depends_on` explicitly; the orchestrator warns about overlaps in the proposals' `Code:` / Impact sections.

## Changes that need the user

An **autonomy level** per change instead of a yes/no flag:

| Level | Orchestrator | Example |
|---|---|---|
| `auto` | runs to merge without gates; stops only at the mandatory calls (`manual-run`, `scope`, `conflict`) | all artifacts done, `openspec validate` passes |
| `review` | runs, gates at proposal and design | `cut-chunks-at-pauses`: proposal only, no specs or design |
| `explore` | does not start it on its own: shows "needs a session with you", or opens the chat in explore mode and waits | `keep-words-after-pauses`: "First an exploration", scope depends on measurements |

How the level is set:

1. **Explicitly**: `/create-change` asks at the end "can this run without you?".
2. **Otherwise from readiness (Definition of Ready)**: missing artifacts → not `auto`; the proposal says "decided after exploration", "TBD" or lists candidates → `explore`; the change touches hotkey, recording, overlay, paste or permissions → flagged up front as "will need your manual run at the end".

Rule: only changes that pass Ready run unattended; everything else is shown to the user, never done silently.

## Where dependencies and level live

OpenSpec drops unknown keys in `.openspec.yaml`. Candidates:

- one `openspec/backlog.yaml` with `name / depends_on / autonomy` entries — the whole graph in one place (leaning);
- an `orchestration.yaml` per change folder — archived together with the change.

## Open questions

1. Launcher: linked desktop sessions or terminal tabs? For linked sessions, how to get `start_session`.
2. Version bump: at merge time, or version-bumping changes one at a time?
3. `explore`-level changes: skip and report, or open the chat and wait?
4. The use case itself: how many changes at once realistically, and is the board inside a chat enough or does it need its own view?

## Placement

The orchestrator skill and changes to `/proceed-change` / `/create-change` go to agent-kit (`kit-placement.md`); this project supplies only its values (`AGENTS.md`, the backlog file, the E2E lock in `justfile`).
