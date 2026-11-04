## 1. Stage budgets and scenario times

- [x] 1.1 Red: the three budget scenarios under `Rule: Each model stage of the check has a time budget` in `dev-tools/tests/features/change_workflow.feature`, steps in `dev-tools/tests/steps/runner/budgets.rs` (the stage-runner steps move into `steps/runner/`, as the steps folder is at its size budget) (a stand-in stage that sleeps, a `stage_budgets` table in the scratch); `just test-one` fails
- [x] 1.2 Green: `stage_budgets` table read in `dev-tools/src/stages/budgets.rs`; the model phase fails a stage past its budget naming the five slowest `scenario time:` lines of the log and the three busiest processes; the unlocked `test` stage prints the warning; the stage table shows the budget
- [x] 1.3 Red: E2E harness unit test in `e2e-test/src/scenario_times.rs` for the five slowest of a list of `scenario time:` lines; `just test-one` fails
- [x] 1.4 Green: the `after` hook of `e2e-test/tests/cucumber_e2e.rs` prints each scenario's time; the run ends with its five slowest
- [x] 1.5 `just static`, `just test`; commit `feat(dev-tools): budget the model stages of check`

## 2. A cheaper smoke set

- [x] 2.1 Red: "A hands-free session not under timing ends before its audio would" in `e2e-test/tests/features/change_workflow.feature` (`@change_workflow @e2e @regression`), steps in `e2e-test/tests/steps/change_workflow.rs` timing the hands-free session against its audio length; `just test-one scenario:` fails (real time)
- [x] 2.2 Green: `Pace` chosen from the scenario tags in the E2E `before` hook (`E2eWorld.voice.pace`); `keep_quiet` and the session's silence in `e2e-test/tests/steps/voice_control/hands_free/mod.rs` and `session/` send a frame every 10 ms at the fast pace; `just e2e-smoke` passes
- [x] 2.3 Red: "A rule with several scenarios runs one in the check" in `change_workflow.feature`, step counting `@smoke` per rule of `local_services.feature` and `speech_recognition.feature` in `dev-tools/tests/steps/runner/smoke_set.rs`; fails
- [x] 2.4 Green: retag the other scenarios of each rule `@regression` in `e2e-test/tests/features/local_services.feature` and `speech_recognition.feature`
- [x] 2.5 Red: the two scan-range scenarios in `change_workflow.feature`, steps in `dev-tools/tests/steps/scan_range.rs`; fails
- [x] 2.6 Green: `dev-tools scan-range` (`dev-tools/src/scan_range.rs`); `_lint` in `justfile` passes its output to gitleaks and trufflehog; `trufflehog_version` 3.99.8
- [x] 2.7 Red: "A change to the stage runner's tests reuses every model stage" in `change_workflow.feature`, steps beside the existing key scenarios; fails
- [x] 2.8 Green: `dev-tools/tests/* code` and `dev-tools/src/journal/* code` in `stage_inputs` of `.config/stages.toml`
- [x] 2.9 `just static`, `just test`; commit `feat(e2e): a faster smoke set and a branch-scoped secret scan`

## 3. A faster unit run

- [x] 3.1 Red: the two per-rule scenarios under `Rule: A failed workflow rule is named and rerun alone` in `change_workflow.feature`, steps in `dev-tools/tests/steps/harness.rs` listing the harness's tests; fails
- [x] 3.2 Green: `rule_tests`, `read_rule_harness_call`, `rule_command_line` moved to `test-steps/src/rule_harness.rs` (re-exported by `e2e-test/src/harness_args.rs`); `dev-tools/tests/cucumber_workflow.rs` lists one test per `Rule:`; `just rerun-failed` names the rule
- [x] 3.3 `just static`, `just test`; commit `feat(dev-tools): one nextest test per rule of the workflow harness`

## 4. Benches in the journal

- [x] 4.1 Red: the three bench scenarios under `Rule: Benches are journaled` in `change_workflow.feature`, steps in `dev-tools/tests/steps/runner/benches.rs`; the journal format scenario expects the 16th field `args`; fails
- [x] 4.2 Green: `bench` and `streaming-bench` in `justfile` run `_stages bench` / `_stages streaming-bench` with `VOICEPUNCHER_BENCH_ARGS`; `dev-tools/src/stages/mod.rs` treats them as model stages without key, pass or reuse; `dev-tools/src/journal/mod.rs` writes `args`; `dev-tools/src/journal/times/` shows the bench time per branch
- [x] 4.3 `just static`, `just test`; commit `feat(dev-tools): journal the benches`

## 5. The needed-tests map and the task-group warning

- [x] 5.1 Red: the five map scenarios under `Rule: An agent runs only the tests of the area it optimizes` in `change_workflow.feature`, steps in `dev-tools/tests/steps/areas.rs`; unit test in `dev-tools/src/areas/mod.rs` over the real `bench_areas` table; fails
- [x] 5.2 Green: `dev-tools needed-tests [path]` and `dev-tools test-area <path>` (`dev-tools/src/areas/`), `bench_areas` in `.config/stages.toml`, `VOICEPUNCHER_TEST_PARTS` in `dev-tools/src/stages/test.rs`, recipes `needed-tests` and `test-area` in `justfile`
- [x] 5.3 Green: the task-group warning in `dev-tools/src/stages/task_groups.rs`, printed by `Runner::start` for `test`, `check` and `full-check`
- [x] 5.4 `AGENTS.md` § TDD and § How to verify a change, README § Tests: `just test-area`, `just needed-tests`, the warning, the budgets, the journaled benches
- [x] 5.5 `just static`, `just test`; commit `feat(dev-tools): the needed-tests map and the task-group warning`

## 6. Budget values and the check

- [ ] 6.1 `VOICEPUNCHER_REUSE_STAGES=0 just check` on an idle machine; `stage_budgets` in `.config/stages.toml` = each model stage's time plus 20 %, `test` 60; `just stage-times 1d` shows the run
- [ ] 6.2 `just static`, `just test`, `just check` (`kit verify`); commit `chore: set the stage budgets`
- [ ] 6.3 No version bump: tooling and tests only; manual checks: none
