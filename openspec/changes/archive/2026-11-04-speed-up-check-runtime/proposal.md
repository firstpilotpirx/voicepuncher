## Why

A change takes far too long from start to merge, and most of that time is spent running tests and benches, not writing code. The target is a strong cut of a change's wall-clock time, not a few seconds off one stage.

### Evidence: the stage journal, 2026-10-17 to 2026-10-31

`~/.cache/voicepuncher/stage-times.jsonl` (2 165 runs; `just stage-times` summarizes it). 127 work branches.

Runs per recipe:

| Recipe | Runs | Failed | Median | p90 | Total | Of it lock wait |
|---|---|---|---|---|---|---|
| `test` | 976 | 407 | 1.9 min | 3.0 min | 33.0 h | 0 |
| `test-one` | 430 | 199 | 0.2 min | 0.5 min | 2.6 h | 0 |
| `full-check` | 335 | 211 | 18.3 min | 63.7 min | 136.9 h | 66.7 h |
| `static` | 160 | 56 | 0.2 min | 0.9 min | 1.1 h | 0 |
| `check` | 78 | 41 | 11.3 min | 43.9 min | 21.7 h | 12.5 h |
| `e2e-smoke` | 39 | 19 | 5.6 min | 56.3 min | 10.6 h | 7.6 h |
| `rerun-failed` | 26 | 5 | 0.3 min | 1.1 min | 2.3 h | 2.2 h |
| `e2e-regression` | 18 | 6 | 1.1 min | 4.8 min | 1.4 h | 1.1 h |
| `e2e-cues` | 16 | 3 | 6.8 min | 10.7 min | 1.8 h | 1.1 h |
| `e2e-resilience` | 10 | 5 | 5.5 min | 44.1 min | 3.1 h | 2.6 h |

- `full-check` is the biggest cost: 137 h in two weeks, half of it waiting for the machine-wide lock, and 63 % of its runs failed (each failure means a fix and another run). Per branch: median 2 full-checks, at most 16.
- `test` runs very often: median 5 per branch, at most 53; 42 % of the runs failed.
- `check` p90 is 44 min although its own work is ~6–9 min: the rest is lock wait behind other sessions' model stages.

Stages that actually ran (not reused), by total time:

| Stage | Ran | Reused | Median | p90 | Total |
|---|---|---|---|---|---|
| `test` | 1 664 | 102 | 84 s | 156 s | 41.1 h |
| `e2e-smoke` (old single smoke stage) | 170 | 0 | 418 s | 544 s | 18.2 h |
| `e2e-seams` | 129 | 5 | 323 s | 406 s | 10.5 h |
| `e2e-bench` (WER on 100 recordings) | 112 | 7 | 222 s | 235 s | 6.7 h |
| `e2e-resilience` | 143 | 14 | 158 s | 205 s | 6.4 h |
| `lint` | 367 | 31 | 18 s | 119 s | 5.9 h |
| `e2e-smoke-voice` | 170 | 30 | 150 s | 201 s | 5.8 h |
| `bundle` | 195 | 18 | 54 s | 156 s | 4.6 h |
| `build-e2e` | 306 | 4 | 13 s | 54 s | 3.1 h |
| `e2e-smoke-services` | 93 | 22 | 122 s | 142 s | 2.9 h |
| `e2e-cues` | 75 | 9 | 132 s | 156 s | 2.7 h |
| `metrics` | 397 | 80 | 8 s | 15 s | 2.6 h |
| `build` | 316 | 4 | 5 s | 54 s | 2.3 h |
| `static` | 276 | 73 | 9 s | 45 s | 1.6 h |
| `refine-eval` | 120 | 21 | 31 s | 63 s | 1.4 h |
| `e2e-regression` | 126 | 20 | 23 s | 46 s | 1.0 h |
| `e2e-smoke-speech` | 74 | 39 | 37 s | 41 s | 0.8 h |
| `e2e-quick` (WER on 10) | 47 | 5 | 47 s | 56 s | 0.6 h |
| `e2e-smoke-refinement` | 50 | 25 | 36 s | 40 s | 0.4 h |
| `e2e-bundle` | 72 | 22 | 10 s | 18 s | 0.3 h |
| `it-real` | 42 | 0 | 8 s | 9 s | 0.1 h |

Fast and frequent: `static`, `fmt`, `metrics`, `build`, `test-one` (seconds). Slow and frequent: `test` (~1.5 min, the most runs and the most total hours), `lint` (up to 2 min cold). Slow and rare: the model stages (`e2e-seams` 5 min, `e2e-bench` 4 min, `e2e-resilience` 2.6 min, `e2e-smoke-voice` 2.5 min, `e2e-smoke-services` 2 min, `e2e-cues` 2.2 min). Reuse of passed stages works for the cheap stages but rarely for the model stages (`e2e-seams` reused 5 of 134).

Most failed tests (each failure costs a fix plus a rerun of the stage):

| Failures | Test |
|---|---|
| 127 | `voicepuncher-e2e-test::cucumber_e2e scenarios` (the whole E2E harness as one test) |
| 55 + 41 | `size_budgets::workspace_stays_within_size_budgets` |
| 30 | `voicepuncher-dev-tools::cucumber_workflow scenarios` |
| 20 | scenario "A hands-free session in a synthesized voice" |
| 20 | `textpasting_macos_it::should_bring_back_copied_text_with_every_type_after_the_paste` |
| 13 | scenario "The user's recorded starts give their cue at once" |
| 13 | scenario "Requests wait their turn when every copy is busy" |

### Evidence: one change, speed-up-streaming-commands (2026-10-30/31)

The change took 8–10 hours of wall clock. Its journaled checks were only 0.6 h (30 `static`, 58 `test-one`, 9 `test`). Most of the time went to unjournaled benches the agent reran after every small fix:

- `just streaming-bench` over all 516 labelled clips in real time: ~20 min a run; with `--fast` ~5 min; over the user's 114 recorded clips in real time ~4 min. Each run takes the machine-wide lock, so two at once queue (a timing run waited > 10 min behind another bench).
- Parameter sweeps (gate gain × threshold: 9 pairs, each a full fast bench plus a recorded bench) ran ~30 min each.
- Each fix was followed by a full real-time bench of both sets (~25 min) instead of the clips the fix touched.
- The same change added an E2E scenario streaming all 114 recordings to `@smoke`, growing `just check` until the user objected; it now runs in `@regression`.

### Why it keeps growing back

- Nothing bounds the time of `@smoke`: a new scenario lands there by default, and `just check` grows with every capability.
- Nothing tells an agent which tests a step needs; agents run whole suites and whole benches after each edit.
- Benches are not journaled, so their time is invisible to `just stage-times` and to audits.
- A failing harness (`cucumber_e2e scenarios`) is one test: one scenario's failure reruns the whole stage.
- Flaky tests (`textpasting_macos_it`, the start-cue and copy-pool scenarios) fail runs that then rerun whole.

## What Changes

- `just check` runs in ≤ 4 min of its own work on an idle machine; `just test` in ≤ 1 min.
- A time budget per model stage of `check` (`stage_budgets` in `.config/stages.toml`: the measured time after the speed-up plus 20 %): a model stage past its budget fails the run naming its five slowest scenarios (the E2E harness prints every scenario's time at its end) and the busiest processes beside it; `test` past its budget prints a warning only, as it runs beside other sessions' builds.
- Voice control smoke scenarios whose latency is not under test (not `@cues`, no wall-clock bound in their steps) stream their audio as fast as the service answers; a scenario that fails at that pace stays in real time and is named in the design.
- `@smoke` keeps one scenario per `Rule:` of `local_services` and `speech_recognition`; the other example scenarios of a rule go to `@regression`.
- `test` gets faster: the `change-workflow` harness runs one nextest test per `Rule:` (as the slice/component harness does). The in-process streaming model tests stay one at a time: they drive the shared command spotting service, which must not see more load than before (`.config/nextest.toml`).
- `lint` scans secrets over the commits since `origin/main` on a branch, the whole history on `main`.
- Finer stage keys: the stage runner's own tests and journal code feed only the code stages.
- `streaming-bench` and `bench` run through the stage runner as journaled runs (recipe, arguments, time, result; never a pass, never reused), so `just stage-times` shows them.
- A needed-tests map for agents: `just needed-tests` prints, per area (crate, module, capability), the tests, scenarios, stages and bench options that cover it; `just test-area <path>` runs the tests of the area that owns the path and prints its stages and bench options; the map's areas are checked by `just test`.
- The stage runner warns when `test`, `check` or `full-check` starts on a branch whose change has an open task group that is not the last: while optimizing one part, run `just test-area`. `AGENTS.md` § How to verify a change states the rule.
- `just stage-times` reports, per branch, the time in checks and in benches apart.
- The pinned `trufflehog` version follows the installed one (3.99.8): the lint guard refuses every run on this machine otherwise.

Current stage: PoC. Target stage: PoC.
Scope: the `check`, `full-check` and `test` stages, their E2E scenarios and tags, the stage runner (budgets, keys, journal of benches, the needed-tests map, the warning), the bench recipes, the agent workflow text in `AGENTS.md`.
Acceptance bar: on an idle machine with nothing reused (`VOICEPUNCHER_REUSE_STAGES=0`) `just check` ≤ 4 min and `just test` ≤ 1 min, as `just stage-times` records; a scenario that pushes a `check` model stage past its budget fails `just check` naming the slowest scenarios; `streaming-bench` runs appear in the stage journal; over the two weeks after the merge, the median check-and-bench time per branch in `just stage-times` is at most half of the 2026-10-17..31 baseline above.
Out of scope: model or recognizer changes, the hosted CI, the release recipe, the flaky tests the journal names (they are frozen when they fail a check of this branch, as the flake rule says, never fixed here), one E2E nextest test per scenario (`just rerun-failed` already reruns only the failed scenarios, and one process per rule would split the shared services and the `@serial` order), a warning in `kit run` (agent-kit is another repository: the stage runner warns instead).

## Capabilities

### Modified Capabilities

- `change-workflow`: stage budgets, bench journaling, the needed-tests map, the task-group warning, faster `check` and `test`.

## Impact

- `justfile`, `dev-tools/` (stage runner, budgets, keys, journal, needed-tests map, warning), `.config/stages.toml`, `.config/nextest.toml`, `e2e-test/tests/features/*.feature` tags, `e2e-test/tests/steps/voice_control/` pacing, `e2e-test/tests/cucumber_e2e.rs` (scenario times), `dev-tools/tests/cucumber_workflow.rs` (per-rule tests), `test-steps/` (the per-rule harness helper shared by both harnesses), `AGENTS.md`.

## Decisions on the open points

- Budget values: the measured time after the speed-up plus 20 %; `test` 1 min, warning only.
- Faster than real time: every voice control scenario not tagged `@cues` and without a wall-clock bound.
- `e2e-smoke-services` and `e2e-smoke-voice` keep their own service starts: the services scenarios test starts and stops, so sharing would test nothing; the budget trims the smoke set instead.
- No E2E nextest test per scenario (out of scope above).
- The needed-tests map is generated from `stage_inputs`, the workspace members and a `bench_areas` table in `.config/stages.toml`, checked by `just test`.

## Release

SemVer: no release — tooling and tests only, no behavior change for the user.
Target version: unchanged.
