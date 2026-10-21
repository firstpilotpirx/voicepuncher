# Design

## Context

Measured on `main` (f004107) before this change: `just test` nextest summary 41.2 s (1237 tests, 339 s of test time on 10 cores). Longest tests: `cucumber_integration component/voice_control` 32.3 s, `engine_it should_keep_the_checks_of_a_long_utterance_few_while_one_waits_for_a_slow_service` 13.5 s, `commandspotting_client_it should_wait_for_a_service_that_answers_pings_when_its_events_come_late_within_the_bound` 12.1 s, `transcription_openai_it should_log_a_copy_hung_…` 11.1 s. Since this change was filed, `reuse-passed-check-stages` split the smoke set into four parts (`e2e-smoke-voice`, `-refinement`, `-speech`, `-services`); each part runs its service binaries one nextest call after another, then the Gherkin harness.

## Decisions

1. **The slice/component harness shows nextest one test per `Rule:`.** A feature file with rules lists one test per rule, named `<feature>/<rule slug>` (lowercase letters, digits and `_`, so nextest filters and `rerun-failed` take it unquoted); scenarios outside any rule, and files without rules, keep the test named by the file. The test runs only the scenarios of its rule. nextest then runs the 31 rules of `voice_control` in parallel processes, so the file no longer bounds `just test`. Rejected: raising cucumber's in-process concurrency (scenarios already run concurrently; the long pole is one file's real-time waits in one process).
2. **Each smoke part runs its service binaries and its Gherkin harness in one nextest call.** nextest runs the binaries beside the harness; the `command-spotting-models` test group still keeps the binaries that drive the shared command spotting service to one test at a time, and the harness keeps its own scenario concurrency. The harness output is captured and printed when it ends (`success-output = "immediate"`), as nextest cannot run tests in parallel with `--no-capture`. A rerun's filter is intersected with the part's selection. Fallback if the shared services do not hold the load (three green runs in the batch `full-check`): `commandspotting_websocket_it` alone before the call.
3. **Shorter real-time waits, same proofs.** `commandspotting_websocket_it` says the sentences or commands of one test on connections of their own at the same time (the service serves several apps at once, which its own test proves) instead of one after another; `commandspotting_client_it`'s late stub answers 6 s late instead of 12 s, still more than the client's 5 s silence limit; the slowest model tests of `engine_it` and the reconnect/hang tests are shortened only where a wait is longer than what it must outlast.
4. **No `@serial` scenario leaves `@smoke`.** Each `@serial` smoke scenario proves an end-to-end behavior (a pasted text, a service start) no other `@smoke` scenario or `just test` test proves at that level, so the proposal's rule moves none.

## Test levels

No spec scenario (`skip_specs: true`): test tooling only.

| Acceptance item | Level |
|---|---|
| one nextest test per rule, scenarios outside rules kept, slug names | unit tests in `e2e-test/src/harness_args.rs` |
| each smoke part's binaries and harness in one nextest call; voice part binaries kept in order of the self-test | recipe self-test `_test-lock-phases` |
| `just test` under 20 s | nextest summary before/after in the PR |
| `e2e-smoke` at most 183 s, every shortened test still proving its behavior | the batch `full-check` stage table; trace table in the PR |
