## Context

`run-e2e-alone-on-the-machine` (archived 2026-10-23) made every model stage of a stage run take one exclusive lock phase (`_e2e-lock`: the machine lock exclusively, then the quiet lock exclusively) and dropped the light/heavy classes. The journal (`_stage-journal-append`) records per lock its class, wait and hold, but not when the hold began, so two runs' holds cannot be checked for overlap from the journal. 29 tests were frozen on 2026-10-23 as load flakes (`.config/frozen-tests.toml`), most before or while that change landed.

## Goals / Non-Goals

**Goals:** the journal proves the exclusivity; the load flakes the exclusive lock cured run again.

**Non-Goals:** changing the lock itself (already exclusive); fixing a test that fails with the machine to itself.

## Decisions

1. **Hold start in the journal.** Each `locks` entry gets `locked`: the local ISO time (`%Y-%m-%dT%H:%M:%S%z`, as `started`) when the hold began, from the report's `locked <class> <epoch>` mark; omitted when the lock was never taken; a caller-held run keeps `caller` and no entry. Rejected: a run-level field — a run has at most one `e2e` hold now, but older lines have several classes.
2. **Overlap report in `just stage-times`.** For every journal line in the period with a `locks` entry of class `e2e` holding `locked` and `held`, the interval is [locked, locked + held]. Pairs of intervals from different runs that intersect by more than 1 s (clock rounding of the 0.1 s marks) are printed under `e2e lock overlaps:` as `<branch> <started> ↔ <branch> <started>: <overlap>`; with none, one line `e2e lock overlaps: none (<n> holds)`. Lines without `locked` (older runs) are left out and counted: `(<k> older holds without a start time)`. Only class `e2e`: the light/heavy classes of older lines were meant to overlap.
3. **Self-test `_test-stage-times`**, run by the `test` stage's self-tests: on a scratch journal (`VOICEPUNCHER_STAGE_JOURNAL`) with three runs, two of them overlapping, it checks the overlap line names the pair and the third run is not named; with the overlapping run removed, it checks `none`; it also runs `_stage-journal-append` on a scratch report with `requested`/`locked`/`returned` marks and checks the line's `locked` time.
4. **Unfreezing.** Every frozen entry is run with `just frozen-tests run` (each in its stage, with its services and the exclusive lock, nothing else on the machine). Each entry whose test passed is removed from `.config/frozen-tests.toml`; an entry whose test failed stays, and `tasks.md` names it with its failure. The unfrozen tests then run in the change's own `check` and the landing `full-check`.

## Risks / Trade-offs

- [A test passes once alone and still flakes later] → the flake rule freezes it again; the acceptance bar measures `e2e-smoke-voice` over the first 20 `full-check` runs after landing.
- [Clock skew between runs] → all runs are on one machine and one clock; the 1 s tolerance absorbs rounding.

## Test levels

No Rust code: the journal and the summary are recipes, proved by a recipe self-test, as for the lock.

| Spec scenario | Level |
|---|---|
| The journal shows the lock wait and hold (hold start) | recipe self-test `_test-stage-times` (scratch report → journal line) |
| The stage summary finds no overlap between runs that took turns | recipe self-test `_test-stage-times` |
| The stage summary names overlapping runs | recipe self-test `_test-stage-times` |
| Two runs' model stages take turns | unchanged, recipe self-test `_test-e2e-locks` |
| A build waits while model stages run | unchanged, recipe self-test `_test-e2e-locks` |
| Model stages wait for running builds and keep new ones out | unchanged, recipe self-test `_test-e2e-locks` |
| Builds still run beside each other | unchanged, recipe self-test `_test-e2e-locks` |
