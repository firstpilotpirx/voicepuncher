---
name: create-change
description: Create an OpenSpec change for later (backlog item) — the proposal, optionally specs, design and tasks — and land it on main without implementing it. Use when the user wants to file a task, feature or bug "for later", "в backlog", "заведи задачу", "создай пропозал".
---

# create-change

Thin wrapper over `opsx:new` and `opsx:continue`. Stages and done criteria: `AGENTS.md` § Workflow.

**Input:** a change name (kebab-case) or a description of what the user wants.

## Rules

- Never write code, tests or `tasks.md` checkmarks; never run `opsx:apply`.
- Never create an artifact the user did not pick in step 1.
- Stop at every artifact gate and wait for the user's approval or edits.
- Write the proposal about *what* and *why*; leave *how* to `design.md`.

## Steps

1. **Pick the artifacts.** Ask the user with `AskUserQuestion` which artifacts to create now:
   - `proposal` only (default)
   - `proposal` + `specs`
   - `proposal` + `specs` + `design`
   - all (`proposal`, `specs`, `design`, `tasks`)
2. **Branch.** `git fetch origin && git switch -c chore/propose-<name> origin/main`.
3. **Proposal.** Run `opsx:new <name>`, then `opsx:continue <name>` once. Show the proposal. **Gate `proposal`.**
4. **Next artifacts.** For each further artifact picked in step 1, in schema order: `opsx:continue <name>`, show it. **Gate `<artifact>`** (`specs`, `design`, `tasks`).
5. **Validate.** When `specs` exist: `openspec validate <name>` passes. A proposal-only change fails validation (no deltas) by design; skip this step for it.
6. **Land on main.** Commit (`docs(openspec): propose <name>`), `just test`, `git push -u origin HEAD`, `gh pr create`, `gh pr merge --squash --delete-branch`.

## Output

Change name, artifacts created, PR link, and the command to continue later: `/proceed-change <name>`.
