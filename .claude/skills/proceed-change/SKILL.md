---
name: proceed-change
description: Drive one OpenSpec change end to end — from an idea or an existing change through proposal, specs, design, tasks, implementation, verification, archive, PR, merge and release — asking at start whether to explore and where to gate, then running every stage. Use when the user says "proceed-change", "доведи задачу", "сделай задачу целиком", "продолжи change <name>".
---

# proceed-change

Thin wrapper over the `opsx:*` skills. Stage order, done criteria and checks: `AGENTS.md` § Workflow, § TDD, § Gherkin, § How to verify a change, § Release. This skill adds only the gates and the resume logic.

**Input:** a change name or a description.

## Rules

- Never show the user anything but the questions in § Start, the gates they picked, the calls in § Call the user and the § Output.
- Never stop at a stage the user did not pick as a gate; run it and go on.
- At a gate or a call: show the result or the problem in ≤ 10 lines, ask with `AskUserQuestion`, send a `PushNotification` when the tool exists, and wait.
- After any gate or interruption, resume by rerunning this skill: derive the stage from the facts in § Resume, never from memory.

## Start

Ask both questions in one `AskUserQuestion` call:

1. Run `explore` first? Yes / no.
2. Gates (the user reviews the result), multi-select: `explore` (only when picked), `proposal`, `specs`, `design`.

## Stages

Run all in order, end to end; skip the ones § Resume shows done.

| # | Stage | How |
|---|---|---|
| 1 | `explore` | `opsx:explore`, only when picked; the gate asks whether the idea is clear |
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
| 12 | `manual-run` | § Call the user |
| 13 | `merge` | `gh pr merge --squash --delete-branch` |
| 14 | `release` | `just release` on `origin/main`, only when `apply` bumped the version |

## Call the user

Always, whatever gates were picked:

| Reason | When |
|---|---|
| `manual-run` | the change touches hotkey, recording, overlay, paste or permissions |
| `scope` | the specs do not settle a decision, or the proposal must change |
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
| PR is merged and the main checkout is at `origin/main` | `merge` |
| tag `v<version in app/Cargo.toml>` exists | `release` |

## Output

At the end: the PR link and the release tag (or "no release").
