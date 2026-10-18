# Tasks

No Rust code and no spec scenario (`skip_specs: true`): each behavior is checked by running recipes with `VOICEPUNCHER_STAGE_JOURNAL` at a scratch file (design.md § Test levels). Scratch edits are never committed. No version bump: test tooling only.

## 1. The journal record (decisions 1–3)

- [x] 1.1 Red: `VOICEPUNCHER_STAGE_JOURNAL=<scratch> just _stages fmt` leaves no scratch file.
- [x] 1.2 Green: the `EXIT` trap of `_stages` appends the record of decision 1 under `flock`; stages are tee'd to a run log and `failed_tests` is parsed from it (decision 3); `VOICEPUNCHER_STAGES_RECIPE` and `VOICEPUNCHER_STAGES_ACCURACY` fill `recipe` and `accuracy`. Check: the 1.1 run leaves one line with every field, `fmt` `passed`; a scratch failing unit test makes `just _stages fmt test lint` record `test` `failed`, `lint` `skipped` and the test's name; INT during `test` records `interrupted`.
- [x] 1.3 Green: `check`, `full-check` (with its accuracy line), `test` and the standalone E2E recipes run through `_stages` naming themselves (decision 2). Check: `just --dry-run` of each shows `_stages` with its stages; `just test` passes and leaves a `test` record; commit.

## 2. The summary (decision 4)

- [x] 2.1 Red: `just stage-times` does not exist.
- [x] 2.2 Green: `stage-times [since]` prints per recipe and stage the runs, median, total and failures; lock wait median and total; failures per test; runs and time per branch; `7d`, a date and the `30d` default select the period, and an empty or missing journal prints one line saying so. Check on the records of group 1; commit.

## 3. Docs and the acceptance runs

- [x] 3.1 `AGENTS.md` § How to verify a change and `README.md` § Testing: the journal, its place and `just stage-times`; the `justfile` comments of `_stages` and the changed recipes.
- [x] 3.2 `just check` passes and leaves a `check` record with every field; a `just full-check` with a scratch failing E2E step (not committed) records `failed`, the failed stage and scenario, then the summary shows both runs; commit.
- [x] 3.3 Version: no bump — test tooling only, no behavior change (proposal § Release); `app/Cargo.toml` stays at 0.18.0.
