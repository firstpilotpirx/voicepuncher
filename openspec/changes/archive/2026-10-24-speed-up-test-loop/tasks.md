## 1. Size budgets as an analyzer

- [x] 1.1 Red: in `dev-tools/tests/features/change_workflow.feature`, under a new `Rule: A fast static check fails before any test runs`, the scenario "A function over budget fails the static check without a test": a scratch workspace with a 41-line function the baseline does not list, `voicepuncher-analyzers size` run there fails naming the function and its length; steps in `dev-tools/tests/steps/analyzers.rs`
- [x] 1.2 Green: move `app/tests/it/size_budgets/` to `analyzers/src/size/` (`voicepuncher-analyzers size [--write-baseline]`, baseline `app/tests/size_baseline.toml`, root from the working directory, the metrics' file walker); fixtures become its unit tests; `syn` and `proc-macro2` pinned at the locked versions; drop `size_budgets` from `app/tests/it/main.rs`; `just size-baseline` calls the analyzer; the budget-scenario steps read `analyzers/src/size/budget.rs`
- [x] 1.3 `just test`; commit

## 2. Metrics over the changed files

- [x] 2.1 Red: scenario "A new complex function in a changed file fails the static check": a scratch git repository whose branch adds a function of cyclomatic complexity 12, `voicepuncher-analyzers metrics --changed main` fails naming it and its complexity
- [x] 2.2 Green: `metrics --changed <base>` (decision 3): changed `.rs` files against the merge base, `rust-code-analysis-cli` on them only, compare only their cyclomatic findings, no report; refuse `--write-baseline`; unit tests for the comparison (new, grown, shrunk, gone)
- [x] 2.3 `just test`; commit

## 3. The static stage

- [x] 3.1 Red: scenarios "A compile error fails the static check" (the `static` recipe runs fmt, clippy, size and changed metrics in that order under `set -e`), "The test recipe stops at the static check" (scratch runner: a failing `_static` leaves `_test` uncalled) and "A clean tree passes the static check in seconds" (the recipe builds and runs no test: no nextest, no `cargo test`, and the size and changed-metrics analyzers finish within 10 s on the repository)
- [x] 3.2 Green: `static` and `_static` in the `justfile`; `test` runs `_stages static test metrics`; `check` and `full-check` run `static` instead of `fmt`; `static` is a code stage in `dev-tools/src/keys/rules.rs` and the stage table
- [x] 3.3 AGENTS.md § How to verify a change and § TDD: `just static` after every edit, `just test` once per task group; the `size_budgets` paths updated; `docs_check` green
- [ ] 3.4 `just test`; commit

## 4. The recording test

- [x] 4.1 Red: scale `should_keep_every_sample_of_a_long_dictation_in_order_on_a_busy_machine` to a 1 s ring and 6 s of audio (decision 4) and prove it still fails: with the drain period raised to 2 s in a scratch edit it drops samples
- [x] 4.2 Green: restore the drain period, the test passes in under 10 s; its comment states the scale; its `.config/nextest.toml` limit, if it has its own, set by the rule of twice the longest run, at least 2 min
- [x] 4.3 `just test`; commit

## 5. Stages only an edit affects

- [x] 5.1 Red: under `Rule: The full check runs only the stages an edit can affect`, the scenarios "An overlay change skips the seams, outages and start cues", "A change to the chunking code runs the seams stage" and "A file no stage names runs every stage", on the stage keys (`dev-tools keys`) of a scratch repository with the repository's `stage_inputs`
- [x] 5.2 Green: the `stage_inputs` lines of decision 5
- [x] 5.3 `just test`; commit

## 6. The seam sample

- [x] 6.1 Red: under `Rule: The chunk seams stage runs a sample unless the chunking code changed`, the scenarios "A branch outside the chunking code runs the seam sample" and "A branch that changes the chunking code runs the whole matrix": `dev-tools seams-stage` on a scratch repository prints the stage and its reason; the full-check recipe prints `● seams:` and runs the stage it names
- [x] 6.2 Green: `dev-tools seams-stage` sharing `accuracy-stage`'s chooser over a table name; `chunking_paths` in `.config/stages.toml`; `full-check` runs the chosen seams stage; `_e2e-seams-sample` with `VOICEPUNCHER_SEAM_SAMPLE=1`; nextest profile `e2e-seams-sample`; the sweep runs lead-in 0 only under the variable; `@seams_sample` Examples blocks of decision 6 in `speech_recognition.feature`; `e2e-seams-sample` in `stage_inputs` wherever `e2e-seams` is
- [x] 6.3 `just test`; commit

## 7. Fewer example rows and the check

- [x] 7.1 Red: a `spec_mirror`-style count in `kit feature-lint` stays green after removing the rows — first remove one row that is the only row of its command in an outline and watch `kit feature-lint` or the reviewer's mapping flag the lost coverage, then put it back
- [x] 7.2 Green: cut the `@regression` rows of `voice_control.feature` to at most 4 per outline (decision 7), keeping one row per command; `spec_mirror` and `kit feature-lint` green
- [x] 7.3 Version: no bump — no behavior change in the app (proposal § Release)
- [x] 7.4 `just static` timed after a one-file edit (≈ 10 s) and `just test` (≈ 45 s) from the stage journal, recorded in the PR body
- [x] 7.5 `just test`; commit
