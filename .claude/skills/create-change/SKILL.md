---
name: create-change
description: Create an OpenSpec change for later (backlog item) — optional exploration, the proposal, optionally specs, design and tasks — and land it on main without implementing it. Use when the user wants to file a task, feature or bug "for later", "в backlog", "заведи задачу", "создай пропозал".
---

# create-change

Thin wrapper over `opsx:explore`, `opsx:new` and `opsx:continue`. Done criteria: `AGENTS.md` § Workflow.

**Input:** a change name (kebab-case) or a description of what the user wants.

## Rules

- Never write code, tests or `tasks.md` checkmarks; never run `opsx:apply`.
- Never run a stage the user did not pick in stage 1; `proposal`, `branch`, `validate` and `land` always run.
- Stop only at the gates the user picked in stage 1; run every other stage without asking.
- Write the proposal from the user's words about *what* and *why*; leave *how* to `design.md`.

## Stages

| # | Stage | Runs | How |
|---|---|---|---|
| 1 | `choose` | always | ask with `AskUserQuestion`, both multi-select: (a) stages to run: `explore`, `specs`, `design`, `tasks`; (b) gates: one per picked stage among `explore`, `proposal`, `specs`, `design`. A picked `tasks` adds the artifacts `openspec status` says it needs |
| 2 | `explore` | when picked | `opsx:explore` |
| 3 | `branch` | always | `git fetch origin && git switch -c chore/propose-<name> origin/main` |
| 4 | `proposal` | always | `opsx:new <name>`, then `opsx:continue <name>` |
| 5 | `specs` | when picked | `opsx:continue <name>` |
| 6 | `design` | when picked | `opsx:continue <name>` |
| 7 | `tasks` | when picked | `opsx:continue <name>` |
| 8 | `validate` | always | `openspec validate <name>` when `specs` exist; a proposal-only change has no deltas and fails by design, so skip it then |
| 9 | `land` | always | commit `docs(openspec): propose <name>`, `just test`, `git push -u origin HEAD`, `gh pr create`, `gh pr merge --squash --delete-branch` |

## Gates

Picked in stage 1. At a gate the user reviews the stage result and approves or edits it before the next stage.

| Gate | Fires |
|---|---|
| `explore` | after stage 2: the user confirms the idea is clear |
| `proposal` | after stage 4 |
| `specs` | after stage 5 |
| `design` | after stage 6 |

## Output

Change name, artifacts created, PR link, and the command to continue later: `/proceed-change <name>`.
