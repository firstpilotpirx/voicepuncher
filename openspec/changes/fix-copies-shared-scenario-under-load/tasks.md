# Tasks

## 1. A fixed request deadline for test clients

- [ ] 1.1 Red: unit test in a new `app/src/adapters/secondary/transcription/openai/deadline.rs` that a fixed deadline replaces the formula for any audio length; run it, see it fail
- [ ] 1.2 Green: move `request_deadline`, `longest_wait`, their constants and unit tests from `openai/mod.rs` into `openai/deadline.rs`, add `OpenAiTranscriber::with_request_deadline(Duration)` and the field `request_transcript` reads; `just test` green, `mod.rs` shorter than its `size_baseline.toml` entry
- [ ] 1.3 `just test`; commit `feat(transcription): fixed request deadline for test clients`

## 2. The shared-requests scenarios under load

- [ ] 2.1 Red: add the scenarios "Requests that arrive at once are shared by the copies while the machine is busy" and "A request without a transcript names the machine's load" to `e2e-test/tests/features/local_services.feature` (`@smoke @serial`) with their steps in a new `e2e-test/tests/steps/local_services/copies/shared.rs` (a `BusyProcessor` guard of `yes` processes held by `PoolRun`, a When sending the requests with a 50 ms deadline, a Then inspecting the sharing check's error); run `just e2e-smoke-services`, see the report scenario fail for the missing load report and note whether the busy one fails on the formula deadline
- [ ] 2.2 Green: move `transcribe`, `short_speech`, `unanswered_at_once`, `ensure_all_answered`, `requests_at_once` and `shared_by_copies` from `copies.rs` into `copies/shared.rs` (`mod shared;` in `copies.rs`); the shared-requests client takes a 30 s deadline (`with_request_deadline`); `ensure_all_answered` appends `load_beside(…, machine_load::report)` to its failure; `just e2e-smoke-services` passes, `copies.rs` shorter than its baseline entry
- [ ] 2.3 `just test`; commit `fix(e2e): shared-copies scenario holds under load and reports the machine load`
- [ ] 2.4 Read the `e2e-smoke-services` time from the stage table; when it passed half of its limit in `.config/nextest.toml`, set the limit to twice the new longest run (whole minutes); `just test` green; commit `test(e2e): smoke-services limit after the under-load scenarios`
- [ ] 2.5 Acceptance bar: start `just e2e-regression` in the main checkout (light lock class) and run `VOICEPUNCHER_REUSE_STAGES=0 just e2e-smoke-services` 5 times in a row in this worktree beside it; all 5 pass (`just stage-times 1d`)
- [ ] 2.6 `just test`; `just check` once (build inputs changed)
- [ ] 2.7 Version: unchanged (`app/Cargo.toml` stays at 0.37.4, no release); manual checks: none
