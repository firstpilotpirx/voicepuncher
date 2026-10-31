## Why

The test loop is too slow and keeps growing back. The stage journal shows `just check` at about 6–9 min with no lock wait, so the time is the work itself: `e2e-smoke-voice` ~2.8 min (synthesized phrases streamed through the models in real time), `e2e-smoke-services` ~2 min, `test` ~1.5–2 min, `lint` ~1 min, `e2e-quick` ~0.8 min. The user has shortened the checks several times, yet each new change adds long E2E scenarios to `@smoke` and `just check` grows again (speed-up-streaming-commands added a scenario over all 114 of the user's recordings, moved to `@regression` only after the user objected). Agents also rerun full suites and full real-time benches after every small fix while tuning, which stretched one change to 8–10 hours.

## What Changes

- `just check` runs in about 4 minutes or less on an idle machine.
- A time budget for the smoke set: a check fails, naming the slowest scenarios, when the smoke set grows past it, so long scenarios go to `@regression` (run by `full-check` only).
- Voice control smoke scenarios whose latency is not under test stream their clips faster than real time.
- The workflow rule for agents: while tuning or optimizing, run only the tests the step needs (`just test-one`, a sampled bench); the full suites and full benches run once, when the feature is done.

Current stage: PoC. Target stage: PoC.
Scope: the `check` stages and their E2E smoke scenarios, the stage runner's budget, the agent workflow text in `AGENTS.md`.
Acceptance bar: `just check` on `main` takes ≤ 4 min wall clock on an idle machine with nothing reused (`VOICEPUNCHER_REUSE_STAGES=0`), as `just stage-times` records; adding a scenario that pushes the smoke set past its budget fails `just check` with the names of the slowest scenarios.
Out of scope: `full-check`'s own stages beyond `check`, model or recognizer changes, the hosted CI.

## Capabilities

### Modified Capabilities

- `change-workflow`: a time budget for the smoke set and the rule to run only the needed tests while tuning.

## Impact

- `justfile`, `dev-tools/` (stage runner, budget), `.config/stages.toml`, `e2e-test/tests/features/*.feature` tags and steps, `AGENTS.md`.

## Open points

- The budget's value and whether it is per stage or for the whole `check` — proposed: per model stage, set to the measured time after the speed-up plus 20 %.
- Which voice control smoke scenarios may stream faster than real time — proposed: every one not tagged `@cues` and not asserting a wall-clock bound.
- Whether `e2e-smoke-services` can share one service start with `e2e-smoke-voice` — proposed: measure first, decide in design.

## Release

SemVer: no release — tooling and tests only, no behavior change for the user.
Target version: unchanged.
