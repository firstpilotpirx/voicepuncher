## Why

On 2026-10-20/21 the same branch passed `full-check` up to four times (`restartable-app-runtime`: 18:08, 23:39, 02:04, 03:51; `settings-window`: four times too) — 14 repeated passing full checks in a day, 13.7 h of calendar time, plus the failed attempts between them. The loop: `kit land` refuses a branch that is behind `main`; `kit sync-base` judges the merge overlapping as soon as both sides touched one build input beyond the version line (`Cargo.lock`, `app/src/main.rs` — nearly every change does); no check carries over, so another `full-check` (~1 h in the lock queue); meanwhile another PR landed (10 in that day), and the branch is behind again. One session ran `kit verify before-merge` seven times. agent-kit already implements batch landing (`kit land` with `landing.batch`: a checked branch joins a landing queue, the leader merges up to 4 queued branches, checks the combination once and lands them all, bisecting a red batch); voicepuncher has no `landing:` block in `.claude/kit.yaml`, so it is off. The user's decision (audit of 2026-10-21): land in groups, one check per group instead of 5–10; a branch tests `main` as of its merge and what lands afterwards is the next lander's business; groups are strictly sequential, no speculative batches for now.

## What Changes

- `.claude/kit.yaml` gets `landing: {batch: true, max_batch: 4}`: `kit land` queues the branch instead of refusing it for being behind `main`; the leader checks the group once and lands it.
- `AGENTS.md` § Workflow stages 9–11 describe landing through the queue: `kit land` after the branch's own check, the group's `full-check` is the merge gate, the session waits for the queue's verdict without polling and does docs or the next change meanwhile.
- The branch's own check before the queue is `just check` (fmt, lint, tests, build, smoke): agent-kit queues a branch with build inputs on `after_group`, and the one `full-check` runs on the group's combined state.

Current stage: PoC. Target stage: PoC.
Scope: `.claude/kit.yaml`, `AGENTS.md`, the `proceed-change` values this project supplies.
Acceptance bar: over a day of orchestrated changes `just stage-times 1d` shows at most one passing `full-check` per landed group and no branch with two passing `full-check` runs; lock wait median below 15 min.
Out of scope: speculative (parallel) batches; the overlap rule itself (agent-kit `parallel-check-churn`); fixing flaky tests.

## Capabilities

### Modified Capabilities

- `change-workflow` (the AGENTS.md workflow): merging goes through the landing queue in groups.

## Impact

- `.claude/kit.yaml`, `AGENTS.md`, `README.md` § Release checklist if it names `kit land`.

## Decisions

- Lands after `eradicate-flaky-tests` and `reuse-passed-check-stages` (both archived on `main`).
- The own check before the queue is `just check`, as agent-kit's batch landing already does (`after_group`).
- No release: workflow only.

## Release

SemVer: no release — no product behavior changes.
Target version: unchanged.
