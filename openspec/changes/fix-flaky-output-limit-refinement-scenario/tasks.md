# Tasks

## 1. The stub records every connection

- [ ] 1.1 Red: move `e2e-test/tests/steps/refinement_service_stub.rs` to `e2e-test/src/refinement_service_stub.rs` (exported from `lib.rs`, the steps and `cucumber_e2e.rs` import it from the crate) and add unit tests there: a model-list request is recorded as an event, a connection without a request line and one with a bad content length are recorded as dropped with their reason; run `just test`, see them fail
- [ ] 1.2 Green: the stub keeps an event log (`models`, `chat` with body, `dropped` with reason) beside `chat_requests`; `read_request` returns the reason it drops a connection; `just test` passes, `just analyzers-baseline` when the `read_request` cyclomatic entry in `.config/analyzers/metrics.toml` drops
- [ ] 1.3 Red: unit test in `e2e-test/src/refinement_service_stub.rs` for the missing-request report: the message names the number of chat requests, then every event in order, then the app's stderr; run `just test`, see it fail
- [ ] 1.4 Green: `missing_request_report(events, stderr)` in the stub module; `only_chat_request` and `expect_message_role` move from `e2e-test/tests/steps/text_refinement/mod.rs` into a new `e2e-test/tests/steps/text_refinement/requests.rs` and the failure uses the report, so `mod.rs` shrinks (`app/tests/size_baseline.toml` entry lowered by `just size-baseline`); `just test` passes
- [ ] 1.5 Add `e2e-test/src/refinement_service_stub.rs` to `stage_inputs` in the `justfile` (stages `e2e-smoke-refinement,refine-eval,e2e-regression`) and `e2e-regression` to the refinement paths there; `just test` passes; commit

## 2. The 20-run series under load

- [ ] 2.1 Red: scenario "Service hits its output limit while the machine is under load" (`@regression`) in `e2e-test/tests/features/text_refinement.feature` with its step in `e2e-test/tests/steps/text_refinement/requests.rs`: the app runs 20 times against one stub, each run checked for the raw transcript and exactly one chat request, the failure carrying the run number and the report; `just e2e-regression` while `just e2e-smoke` runs from this worktree (or another session's check holds the heavy class) — the Red run is the series itself
- [ ] 2.2 Green: fix what a failed run of 2.1 names (stub, step or the app's file-mode path) and run 2.1 again until 20 runs pass; when the first series passes, record in `design.md` that no run failed
- [ ] 2.3 Set the `e2e-regression` limit in `.config/nextest.toml` to twice its new longest run (from `just stage-times 1d`) when the stage passed half of its 2 min; update the recipe comment above `e2e-regression` in the `justfile` (it now runs the refinement series too) and the `@regression` line in `AGENTS.md` § Gherkin; `just test` passes; commit

## 3. Final check

- [ ] 3.1 `just check` once (build inputs changed); `version` in `app/Cargo.toml` stays 0.37.3 unless 2.2 changed the app (then PATCH to 0.37.4 and the proposal's target version); manual checks: none (file mode and tests only)
