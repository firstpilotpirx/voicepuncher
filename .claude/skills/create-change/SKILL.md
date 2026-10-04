---
name: create-change
description: Create an OpenSpec change for later (backlog item) — optional exploration, the proposal, optionally specs, design and tasks — and land it on main without implementing it. Use when the user wants to file a task, feature or bug "for later", "в backlog", "заведи задачу", "создай пропозал".
---

# create-change

Thin wrapper over `opsx:explore`, `opsx:new` and `opsx:continue`. Done criteria: `AGENTS.md` § Workflow.

**Input:** a change name (kebab-case) or a description of what the user wants.

## Rules

- Never write code, tests or `tasks.md` checkmarks; never run `opsx:apply`.
- Never create an artifact the user did not pick in stage 1.
- Stop only at the gates in § Gates; run every other stage without asking.
- Write the proposal from the user's words about *what* and *why*; leave *how* to `design.md`.

## Stages

| # | Stage | How |
|---|---|---|
| 1 | `choose` | ask with `AskUserQuestion`: explore first (yes / no), and the artifacts: `proposal` only (default) / + `specs` / + `specs` + `design` / all (+ `tasks`) |
| 2 | `explore` | `opsx:explore`, only when picked in stage 1 |
| 3 | `branch` | `git fetch origin && git switch -c chore/propose-<name> origin/main` |
| 4 | `proposal` | `opsx:new <name>`, then `opsx:continue <name>` |
| 5 | `specs` | `opsx:continue <name>`, only when picked |
| 6 | `design` | `opsx:continue <name>`, only when picked |
| 7 | `tasks` | `opsx:continue <name>`, only when picked |
| 8 | `validate` | `openspec validate <name>`, only when `specs` exist (a proposal-only change has no deltas and fails by design) |
| 9 | `land` | commit `docs(openspec): propose <name>`, `just test`, `git push -u origin HEAD`, `gh pr create`, `gh pr merge --squash --delete-branch` |

## Gates

The user reviews the artifact and approves or edits it before the next stage.

| Gate | Fires |
|---|---|
| `explore` | after stage 2: the user confirms the idea is clear |
| `proposal` | after stage 4 |
| `specs` | after stage 5 |
| `design` | after stage 6 |

## Output

Change name, artifacts created, PR link, and the command to continue later: `/proceed-change <name>`.
