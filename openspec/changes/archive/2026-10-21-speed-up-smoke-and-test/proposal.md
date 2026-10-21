## Why

`speed-up-tests` (PR #246, archived 2026-10-20) halved `just test` and shortened `refine-eval` and `e2e-resilience`, but left two of its first targets open: `just test` under 20 s and `e2e-smoke` in `check` at most half of its 365 s baseline. Measured after #246:

- `just test` takes 46–47 s (nextest summary). The long pole is `cucumber_integration component/voice_control` (32 s: one feature file runs as one test), then the command spotting model tests (`placed_commands_it` 12–17 s, `engine_it` 13 s, `commandspotting_client_it` 12 s).
- `e2e-smoke` takes 411–440 s. The service binaries run one nextest invocation after another before the Gherkin harness: `transcription_openai_it` 4.5 s, `textrefinement_openai_it` 1 s, `commandspotting_websocket_it` 94 s (`should_ignore_a_command_phrase_with_more_speech_after_it` 37.8 s, the 48 kHz test 17.9 s), `voice_forms_it` 25 s, `placed_commands_it` 19 s, `voice_samples_it` 12 s. Then the `@smoke` Gherkin scenarios take 234–243 s at concurrency 6, bounded by the `@serial` scenarios that stream audio in real time.

## What Changes

- `e2e-smoke` runs its service binaries beside the Gherkin harness instead of one after another, as far as the shared test services hold the load.
- The slowest real-time waits of the `e2e-smoke` service binaries (above all `commandspotting_websocket_it`) get shorter, keeping what each test proves.
- `just test` no longer waits for one 32 s feature file: `component/voice_control` runs as several nextest tests (e.g. one per `Rule:`), and the slowest command spotting model tests are shortened.
- `@serial` real-time scenarios that `full-check` also runs are weighed for moving out of the `@smoke` set.

Current stage: PoC. Target stage: PoC.
Scope: tests, the Gherkin harnesses, `justfile` stage recipes and `.config/nextest.toml` only.
Acceptance bar: `just test` under 20 s (nextest summary) and `e2e-smoke` in `check` at most 183 s (half of the 365 s baseline), both over three green runs; every behavior a moved or shortened test proved is still proved (traced in the PR); `just full-check` passes; before/after stage tables in the PR.
Out of scope: product behavior; fixing flaky tests; smaller accuracy samples.

## Capabilities

### Modified Capabilities

- none: which tests run where and how fast changes no spec, unless a timing must become settable (then the spec states it by what it outlasts, as `speed-up-tests` did).

## Impact

- `justfile` (`_e2e-smoke`, `e2e_smoke_binaries`), `.config/nextest.toml`, `app/tests/cucumber_integration.rs`, `e2e-test/src/harness_args.rs`, `app/tests/commandspotting_websocket_it.rs`, `command-spotting/tests/placed_commands_it.rs`, `command-spotting/tests/engine_it.rs`, `app/tests/commandspotting_client_it.rs`, `e2e-test/tests/features/*.feature`.

## Open points

- Whether the shared test services hold the service binaries and the harness at once: proposed — measure three green runs; fall back to running the light binaries beside the harness and `commandspotting_websocket_it` alone.
- Whether 20 s for `just test` is reachable with the real-model tests in it: proposed — try per-Rule splitting and shorter waits first; if not, state the floor in the PR and agree a bar.
- Which `@serial` smoke scenarios may move to `full-check`: proposed — only those whose behavior another `@smoke` scenario or a `just test` test also proves.

## Release

SemVer: no release — tests, harness and recipes only, no behavior change.
Target version: none — the version stays.
