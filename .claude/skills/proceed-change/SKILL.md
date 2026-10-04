---
name: proceed-change
description: Drive one OpenSpec change end to end — from an idea or an existing change through proposal, specs, design, tasks, implementation, verification, archive, PR, merge and release — stopping at the gates the user picked and calling the user when a decision is theirs. Use when the user says "proceed-change", "доведи задачу", "сделай задачу целиком", "продолжи change <name>".
---

# proceed-change

Thin wrapper over the `opsx:*` skills. Stage order, done criteria and checks: `AGENTS.md` § Workflow, § TDD, § Gherkin, § How to verify a change, § Release. This skill adds only the gates and the resume logic.

**Input:** a change name or a description, plus optional `--stop <gate,…>` (§ Gates). Without `--stop`, stop at `proposal`, `specs`, `design`.

## Rules

- Never start `apply` before the `proposal` artifact is approved by the user in this or an earlier session.
- At a gate or a call to the user: stop, show the artifact or the problem in ≤ 10 lines, ask with `AskUserQuestion`, send a `PushNotification` when the tool exists, and wait.
- After any gate or interruption, resume by rerunning this skill: derive the stage from the facts in § Resume, never from memory.

## Work stages

| # | Stage | How |
|---|---|---|
| 1 | `explore` | `opsx:explore`, only when the user asks for it |
| 2 | `proposal` | `opsx:new`, then `opsx:continue` |
| 3 | `specs` | `opsx:continue` |
| 4 | `design` | `opsx:continue` |
| 5 | `tasks` | `opsx:continue` |
| 6 | `branch` | `git fetch origin && git switch -c <type>/<name> origin/main`; commit the planning artifacts |
| 7 | `apply` | `opsx:apply` with `tdd`; commit and push after every task group |
| 8 | `verify` | `opsx:verify`, then the `spec-test-reviewer` subagent |
| 9 | `archive` | `opsx:archive`; commit and push |
| 10 | `full-check` | `just full-check`, only when a build input changed |
| 11 | `pr` | `git push -u origin HEAD`, `gh pr create` |
| 12 | `manual-run` | the user runs the app (`AGENTS.md` § How to verify a change) |
| 13 | `merge` | `gh pr merge --squash --delete-branch` |
| 14 | `release` | `just release` on `origin/main`, only when `apply` bumped the version |

## Gates

The user picks them with `--stop`.

| Gate | Default | Fires |
|---|---|---|
| `proposal` | on | after stage 2 |
| `specs` | on | after stage 3 |
| `design` | on | after stage 4 |
| `tasks` | off | after stage 5 |
| `pr` | off | after stage 11, before stage 12 |

## Call the user

Always on; `--stop` never turns them off.

| Reason | Fires |
|---|---|
| `scope` | the specs do not settle a decision, or the proposal must change |
| `manual-run` | the change touches hotkey, recording, overlay, paste or permissions |
| `conflict` | merging `origin/main` changes the meaning of the change, not only text |

## Resume

Find the first stage whose fact is missing and start there:

| Fact | Stage done |
|---|---|
| `openspec/changes/<name>/proposal.md` exists | `proposal` |
| `openspec status --change <name>` shows the artifact done | `specs`, `design`, `tasks` |
| branch `*/<name>` exists | `branch` |
| every task in `tasks.md` is `[x]` | `apply` |
| `openspec/changes/archive/*-<name>/` exists | `verify`, `archive` |
| PR for the branch exists | `pr` |
| PR is merged | `merge` |
| tag `v<version in app/Cargo.toml>` exists | `release` |

## Output

At the end: the stages that ran, the gates that stopped, the PR link and the release tag (or "no release").
