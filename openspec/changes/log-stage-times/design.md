# Design

## Decisions

### 1. One JSON line per run, appended by `_stages`

`_stages` already knows every stage's mark and time, the lock wait and the lock hold, and its `EXIT` trap runs on success, failure and Ctrl-C/TERM. The trap appends one JSON object per run to `VOICEPUNCHER_STAGE_JOURNAL` (default `~/.cache/voicepuncher/stage-times.jsonl`, outside every worktree, beside the speech cache): `started` (ISO 8601, local offset), `seconds`, `worktree`, `branch`, `commit`, `recipe`, `result` (`passed`, `failed`, `interrupted`), `exit`, `stages` (each `name`, `result`, `seconds`), `lock_wait` and `lock_held` (seconds, `null` when not reached, `"caller"` under a caller's lock), `accuracy` (the `full-check` choice line, else `null`) and `failed_tests`. A stage requested but never reached is `skipped`; `reused` is reserved for `reuse-passed-check-stages`. Perl writes the line (core `JSON::PP`, already the clock of `_stages`) under `flock`, so parallel runs never interleave lines. A run killed with KILL leaves no record: no trap runs.

*Alternatives:* a file per run (harder to summarize); SQLite (a new tool); a Rust binary (a build before every record).

### 2. Every check-like recipe goes through `_stages`

`test` and the standalone E2E recipes (`e2e`, `e2e-smoke`, `e2e-quick`, `e2e-seams`, `e2e-bench`, `e2e-resilience`, `refine-eval`) become `_stages <their stages>`: one place records every run, and they gain the stage table. `_stages` already runs an unlocked stage through the wrapper with a lock directory of the run's own (what `test` does today) and model stages under the machine lock with the services started once (what each E2E recipe does on its own). The caller names itself in `VOICEPUNCHER_STAGES_RECIPE` (default `stages`, for `just _stages …` by hand); `full-check` passes its accuracy line in `VOICEPUNCHER_STAGES_ACCURACY`.

*Alternatives:* a record call in each recipe (eight copies of the timing logic).

### 3. Failed test names come from the stage's output

Each stage's stdout and stderr are also copied to a log of the run (`tee`); when the run ends, the trap collects nextest's `FAIL`, `TIMEOUT` and `SIG*` result lines and, for the Gherkin harness, each cucumber scenario with a failed step, deduplicated, as `failed_tests`. The stage's exit code stays its own. Behind `tee` cargo and nextest no longer see a terminal, so when the run's stdout is one, `_stages` exports `CARGO_TERM_COLOR=always` and the parser strips ANSI codes.

*Alternatives:* nextest JUnit reports (one file per nextest call, overwritten within a stage, and no scenario names for the harness).

### 4. `just stage-times [since]` prints the summary

A public recipe reads the journal with Perl and prints, for runs since a date (`2026-10-01`) or a span (`7d`, default `30d`): per recipe and stage the runs, median and total seconds and failures; lock wait median and total; failures per test, most first; runs and total time per branch (a change's branch is `<type>/<change>`, so runs per change come from the branch name). Plain aligned text, no colors (`terminal-output.md`).

## Test levels

No spec scenario (`skip_specs: true`) and no Rust code: each acceptance item is checked by running the recipes with `VOICEPUNCHER_STAGE_JOURNAL` at a scratch file (tasks.md).

| Acceptance item | Level |
|---|---|
| a passing `just check` leaves a record with every field | recipe run |
| a failed `just full-check` records the failed stage, skipped stages and failed tests | recipe run (scratch failing test) |
| an interrupted run is recorded as `interrupted` | recipe run (INT) |
| the summary prints medians, totals, lock wait and failures per test for a period | recipe run on the records above |
| the journal survives worktree removal | by placement (`~/.cache`) |

No manual run: hotkey, recording, overlay, paste and permissions are untouched.

## Dependencies

None declared: this change lands first so the speed-up changes can show before/after numbers; they edit `_stages` too and merge on top of it.
