---
name: create-change
description: Create an OpenSpec change for later (backlog item) — optional exploration, the proposal, optionally specs, design and tasks — and land it on main without implementing it. Use when the user wants to file a task, feature or bug "for later", "в backlog", "заведи задачу", "создай пропозал".
---

# create-change

Thin wrapper over `opsx:explore`, `opsx:new` and `opsx:continue`. Done criteria: `AGENTS.md` § Workflow.

**Input:** a change name (kebab-case) or a description of what the user wants.

## Rules

- Never write code, tests or `tasks.md` checkmarks; never run `opsx:apply`.
- Never show the user anything but the two questions in § Start, the gates they picked and the § Output.
- Never run a stage the user did not pick; `proposal` always runs.
- Stop only at the gates the user picked; run everything else without asking.
- Write the proposal from the user's words about *what* and *why*; leave *how* to `design.md`.

## Start

Ask both questions in one `AskUserQuestion` call, both multi-select:

1. Stages to run: `explore`, `specs`, `design`, `tasks`.
2. Gates (the user reviews the result): `explore`, `proposal`, `specs`, `design`.

A picked `tasks` adds the artifacts `openspec status` says it needs.

## Stages

Run in this order; at a picked gate show the result and wait for approval or edits.

| Stage | How |
|---|---|
| `explore` | `opsx:explore`; the gate asks whether the idea is clear |
| `proposal` | `opsx:new <name>`, then `opsx:continue <name>` |
| `specs` | `opsx:continue <name>` |
| `design` | `opsx:continue <name>` |
| `tasks` | `opsx:continue <name>` |

## Silent steps

- Before `proposal`: `git fetch origin && git switch -c chore/propose-<name> origin/main`.
- After the last stage: `openspec validate <name>` when `specs` exist (a proposal-only change has no deltas and fails by design); commit `docs(openspec): propose <name>`, `just test`, `git push -u origin HEAD`, `gh pr create`, `gh pr merge --squash --delete-branch`.

## Output

Change name, artifacts created, PR link, and the command to continue later: `/proceed-change <name>`.
