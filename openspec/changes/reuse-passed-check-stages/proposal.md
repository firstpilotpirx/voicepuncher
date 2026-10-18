## Why

The user wants more changes done in the same time without losing quality, and tests are long while changes are many: the number of test runs per change must go down. Research of 2026-10-18 (report https://claude.ai/artifact/HaDTCo6LEo833xZijMqCh7) over 25 sessions and 140 recorded runs of `just check` / `just full-check` (14–18 Oct) counted about 10 check-level runs per merged change (13 code changes merged), where the workflow plans two (`just check` after the last task group, `just full-check` before merge). Causes:

- `just full-check` repeats every stage `just check` already passed on the same commit: fmt, lint, `test`, build, build-e2e, `e2e-smoke` (269–343 s) and `refine-eval` — about 6 minutes of repeated work per change.
- `main` moves while a change waits to merge: `kit land` refused 21 times because origin/main had commits the branch had not merged; after `kit sync-base` a whole check ran again, also when the merged commits touched none of a stage's inputs.
- Failed runs rerun every stage again, including the stages that had passed (flakes are fixed in `fix-flaky-tests`).

## What Changes

- A stage that passed on exactly the same inputs is not run again: `just full-check` after a passing `just check` on the same commit runs only the stages `check` does not have (`e2e-seams`, `e2e-resilience`, the 100-recording accuracy sample when chosen) and reports the reused stages as passed earlier, with the run they came from.
- After merging `main` (`kit sync-base`), only the stages whose inputs the merged commits changed run again; a stage whose inputs did not change keeps its earlier pass.
- A rerun after a failure runs the failed stage and the stages after it, not the stages that already passed on the same inputs.
- A stage that passed on `main` (or in any worktree of this machine) on exactly the same inputs is not run again on a branch (user agreed 2026-10-18): a change that touches only text refinement does not rerun the voice control scenarios whose files and services it did not touch. The record of passed stages is shared across worktrees, keyed by the stage's input hash.
- Quality stays: a stage is reused only when every input it depends on is unchanged (code, tests, features, recipes, config, toolchain, models and corpus); when its inputs cannot be told, the stage runs.

Current stage: PoC. Target stage: PoC.
Scope: the `justfile` stage runner (`_stages`, `check`, `full-check`), a record of passed stages keyed by their inputs, AGENTS.md § How to verify a change; the parts of agent-kit (`kit sync-base`, `kit verify`, `checks.disjoint_merge`) that decide which check to run after a merge, if the repository alone cannot do it.
Acceptance bar: on a change that runs `just check` then `just full-check` on one commit, the full check runs only its extra stages; after a `kit sync-base` that merged commits touching only `openspec/` or docs, no model stage runs again; a change to an input of a stage makes that stage run; check-level runs per merged change measured before and after.
Out of scope: fixing flaky tests (`fix-flaky-tests`); making stages themselves faster (`speed-up-tests`); dropping any stage or test.

## Capabilities

### Modified Capabilities

- none: tooling only (test recipes and check workflow); no product capability changes.

## Impact

- `justfile` (`_stages`, `_model-stages`, `check`, `full-check`), a cache directory for stage results (e.g. under `target/` or `~/.cache/voicepuncher/`), `AGENTS.md` § How to verify a change, `.claude/kit.yaml` `checks`.
- Possibly agent-kit: `kit sync-base`, `kit verify`, `kit land`.

## Open points

- What a stage's inputs are: proposed — a hash of the tracked files the stage builds or reads (per stage path lists, like `recognition_paths`), plus the toolchain, nextest version, model files and corpus; everything else counts as an input of every stage until listed.
- How long a recorded pass is trusted: proposed — any age, while its inputs hash the same.
- Whether `kit` or the `justfile` decides what runs after `kit sync-base`: proposed — the `justfile` skips stages by input hash, `kit sync-base` keeps naming the check; a kit change only if needed (`~/.claude/rules-custom/kit-placement.md`).
- Whether a stage passed in another worktree on the same inputs counts: decided — yes (user, 2026-10-18), including passes on `main`. Open: how fine-grained stage inputs must be for a change to skip a stage (proposed — split `e2e-smoke` by capability: voice control, speech recognition, refinement, local services, each with its own path list).
- Where the record lives: proposed — a machine-wide directory (e.g. `~/.cache/voicepuncher/stage-passes/`), written only by a passing stage, never by hand.
- Target version: proposed no release — tooling only.

## Release

SemVer: no release — test tooling only; no behavior change.
Target version: 0.18.0.
