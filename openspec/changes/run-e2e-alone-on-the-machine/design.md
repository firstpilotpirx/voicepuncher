## Context

The `justfile` splits the model stages of a stage run into three lock classes (`split-machine-lock`): `light` (voice control: `e2e-smoke-voice`, `e2e-regression`, `e2e-resilience`) and `heavy` (every other model stage) each take a class lock and then the machine lock in its shared mode, so one run's light phase runs beside another run's heavy phase; `exclusive` (`e2e-cues`) takes the machine lock exclusively and then the quiet lock exclusively, which every build stage of `_stages` (fmt, lint, test, metrics, build, bundle, build-e2e, standalone `just test` too, as it runs through `_stages`) holds in the shared mode. Load-sensitive scenarios of the light and heavy phases flaked under that overlap: `full-check` passed 11 of 27 runs on 2026-10-22.

## Goals / Non-Goals

**Goals:** every model stage runs with the machine to itself; builds and tests of other runs wait while it runs; the stage table and the journal keep showing lock wait and hold.

**Non-Goals:** fixing individual flaky tests; the standalone locked recipes outside `_stages` (`build`, `bench`, `bench-smoke`, `test-real-keyboard`), which already take the machine lock exclusively; `cargo watch`.

## Decisions

1. **One lock phase for all model stages.** `_stages` runs every model stage it has to run in one phase under the lock the `exclusive` class takes today: the machine lock exclusively (outer wrapper, deadline = `MACHINE_LOCK_WAIT` + deadline), then the quiet lock exclusively (inner wrapper, the phase's deadline). The recipe `_class-lock <class>` becomes `_e2e-lock <command…>`; `_lock-phases`, `light_stages`, `exclusive_stages` and the `light`/`heavy` class lock directories go. Rejected: keeping three classes and only making `light` and `heavy` exclusive — same effect, three code paths; one phase per stage — a run would give the lock back between its stages and wait in the queue again for each.
2. **Order inside the phase:** the order of `check_model_stages` and `full_check_model_stages` as it is (voice control first, `e2e-seams` late, `e2e-cues` last), so fail-fast keeps showing the most frequent failure first. The order self-test (`_test-lock-phases`) becomes `_test-stage-order` and checks those lists directly.
3. **One port set.** Every model stage runs on the test ports (28081, 28085, 28080, 28082); the light ports (27081, 27085, 27080, 27082) and their `VOICEPUNCHER_TEST_LIGHT_*` variables go, `heavy_ports` becomes `test_ports`. The services start once for the phase (`_model-stages`), as the exclusive phase did.
4. **Journal and table.** The phase's class is `e2e`: one row `e2e lock  wait …, held …` and one `locks` entry `{class: "e2e", …}` per run; `lock_wait` and `lock_held` keep their meaning, so `just stage-times` needs no change. A run whose caller already holds the machine lock keeps its `machine` phase.
5. **Standalone `just test` waits.** It already runs its stages through `_quiet-lock` (shared), so it waits for a running or waiting model phase without new code; `AGENTS.md` stops calling it lock-free. `cargo watch` (the user's app) takes no lock.
6. **Non-load-sensitive stages are exclusive too** (`e2e-bench`, `refine-eval`, `e2e-bundle`, `e2e-quick`): one rule, no list to keep current.
7. **Self-test of the lock** (`_test-class-locks` → `_test-e2e-locks`, on scratch lock directories): two model phases take turns; an exclusive run of the machine lock (another project, a standalone recipe) keeps a model phase out; two quiet builds overlap; a model phase waits for the running quiet build and keeps a later one out; the lock wait stays 6 h unless the caller sets it.
8. **Acceptance series.** After the last task group, `kit verify --series 2` of the level `kit checks-needed` names, beside the other sessions' queue; the landing group's `full-check` is the third run. Queue time is measured a day after the merge with `just stage-times` (open point 1 of the proposal: accepted).

## Risks / Trade-offs

- [Lock queue grows: every model phase now waits for every other, and builds of all sessions stop for up to a `full-check`'s model phase, ≈ 16 min] → accepted by the proposal; measured with `just stage-times` after a day.
- [A long model phase under one deadline] → the inner deadline (`MACHINE_LOCK_TIMEOUT`, 1 h default) covers the whole phase; `full-check`'s phase is ≈ 16 min.
- [Builds starve a waiting model phase] → the kit lets a waiting exclusive holder go before new shared ones (proved by the self-test).

## Test levels

No Rust code changes and no Gherkin scenario: the lock is a recipe, proved by the recipe self-tests the `test` stage runs, as for `split-machine-lock`.

| Spec scenario | Level |
|---|---|
| Two runs' model stages take turns | recipe self-test `_test-e2e-locks` (scratch lock directories) |
| A build waits while model stages run | recipe self-test `_test-e2e-locks`: a quiet build started during a model phase begins after it |
| Model stages wait for running builds and keep new ones out | recipe self-test `_test-e2e-locks` |
| Builds still run beside each other | recipe self-test `_test-e2e-locks`: two quiet builds overlap |
| The journal shows the lock wait and hold | measurement: the `e2e lock` row and the journal's `locks` entry of the series runs (decision 8) |
