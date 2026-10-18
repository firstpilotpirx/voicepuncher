## Why

The research of 2026-10-18 (report https://claude.ai/artifact/HaDTCo6LEo833xZijMqCh7) that found where test time goes had to dig the stage tables of `just check` / `just full-check` out of 51 session transcripts and 119 scratchpad logs by hand, then deduplicate them to 140 runs. The table each check prints at its end (`stages:` — each stage's time, lock wait, lock held, total) is lost once the session ends. Six changes now aim to speed up the tests (`fix-flaky-tests`, `speed-up-tests`, `reuse-passed-check-stages`, `split-machine-lock`, `rerun-only-failed-tests`, `fail-fast-stage-order`, and agent-kit `batch-merge-related-changes`); without a lasting record nobody can show they worked or find the next bottleneck. The user wants a permanent journal of stage times to keep tuning the tests.

## What Changes

- Every run of `just check`, `just full-check` and every standalone E2E recipe appends one record to a machine-wide journal: when it ran, worktree and branch, commit, which recipe, each stage with its time and result (passed, failed, skipped or reused), lock wait and lock held, the accuracy sample chosen, and the failed tests' names.
- A failed or interrupted run is recorded too, so time lost to failures and reruns can be counted.
- A recipe prints a summary of the journal for a period: time per stage (median and total), lock wait, failures per stage and per test, runs per merged change.

Current stage: PoC. Target stage: PoC.
Scope: `justfile` (`_stages`, `_model-stages`, the E2E recipes, a new summary recipe), AGENTS.md § How to verify a change.
Acceptance bar: after a `just check`, a failed `just full-check` and an interrupted run, the journal holds three records with every field above; the summary recipe prints per-stage medians and totals, lock wait and failures per test for a chosen period; the journal survives worktree removal.
Out of scope: changing what any stage runs or how long it takes; dashboards or charts (the summary is text).

## Capabilities

### Modified Capabilities

- none: test tooling only; no product capability changes.

## Impact

- `justfile`, AGENTS.md § How to verify a change.
- A journal file outside every worktree (e.g. `~/.cache/voicepuncher/stage-times.jsonl`).

## Open points

- Journal format and place: proposed — one JSON line per run in `~/.cache/voicepuncher/stage-times.jsonl`, shared by every worktree.
- Whether `just test` runs are recorded too: proposed — yes, as a one-stage run, since it runs after every edit.
- How "runs per merged change" is counted: proposed — by branch name, matched to merged PRs later.
- Order with the speed-up changes: proposed — land this first, so the others can show before/after numbers; it touches the end of `_stages`, which they also change (declare a dependency in them when they start, or merge carefully).
- Whether the generic part belongs to agent-kit (`kit audit` already reads sessions): proposed — keep it in the `justfile` now.
- Target version: proposed no release — tooling only.

## Release

SemVer: no release — test tooling only; no behavior change.
Target version: 0.18.0.
