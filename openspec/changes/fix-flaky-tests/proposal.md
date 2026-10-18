## Why

Checks waste about a third of their time on failed runs that are then rerun. Research of 2026-10-18 (report https://claude.ai/artifact/HaDTCo6LEo833xZijMqCh7) read the stage tables of 140 `just check` / `just full-check` runs from session transcripts and scratchpad logs (14–18 Oct): 294 of 954 minutes were spent in runs that failed. Failed stages: `e2e-smoke` 52 times, `e2e-seams` 10, `test` 5, `fmt` 3, `refine-eval` 2, `e2e-bench` 1. The audit of 2026-10-16 already saw `e2e-smoke` red in 44 of 80 runs and `e2e-seams` red in 18 of 18. The user's rule: when the behavior a test checks is fine, the test must never fail. Every rerun also holds the machine-wide lock, so other sessions wait too.

Tests that failed most often in session transcripts of 9–18 Oct (153 distinct failures counted; some may be expected red runs during development, see Open points):

| Failures | File | Test |
|---|---|---|
| 26 | `app/tests/voice_forms_it.rs` | `should_carry_out_every_recorded_way_of_saying_the_commands` (typically "1 of 82 recorded forms went wrong") |
| 13 | `app/tests/chunked_transcription_it.rs` | `should_merge_chunked_speech_without_repeating_or_dropping_a_phrase` |
| 12 | `app/tests/transcription_openai_it.rs` | `should_transcribe_corpus_sample_through_real_service` |
| 11 | `app/tests/commandspotting_websocket_it.rs` | `should_serve_several_apps_listening_at_once` |
| 10 | `app/tests/voice_samples_it.rs` | `should_recognize_every_recorded_command_of_the_user` (typically "1 of 37 recorded commands missed") |
| 7 | `app/tests/commandspotting_websocket_it.rs` | `should_recognize_each_command_streamed_from_a_48_khz_microphone` |
| 6 | `app/tests/commandspotting_websocket_it.rs` | `should_recognize_commands_after_long_listening_with_the_phrase_set_switched` |
| 5 | `app/tests/commandspotting_websocket_it.rs` | `should_report_a_spoken_phrase_between_the_bounds_of_its_speech` |
| 4 | `command-spotting/tests/engine_it.rs` | `should_spot_speech_over_room_noise` |
| 3 | `command-spotting/tests/engine_it.rs` | `should_keep_the_users_two_commands_apart_across_a_short_pause_with_a_click` |
| 3 | `app/tests/commandspotting_websocket_it.rs` | `should_ignore_a_command_phrase_said_inside_a_sentence_after_long_listening` |
| 2 | `app/tests/commandspotting_reconnect_it.rs` | `should_be_back_within_seconds_once_the_service_answers_after_a_long_outage` |
| 2 | `app/tests/commandspotting_client_it.rs` | `should_wait_for_a_service_that_answers_pings_however_late_its_events_come` |
| 2 | `app/tests/commandspotting_websocket_it.rs` | `should_ignore_a_command_phrase_with_words_only_before_it` |

The `@smoke` Gherkin run of `e2e-smoke` also failed with a few of its ~178 scenarios red; which ones is not in the counted lines. One session ran `just full-check` 4 times in 10 minutes on unchanged code, failing each time on `voice_forms_it` or `should_serve_several_apps_listening_at_once`, then passing.

## What Changes

- Every failure of a test stage found in past run logs and session transcripts is traced to its cause: a real defect, a race or timing effect in the test or the code (real-time audio pacing, service warm-up, timeouts, shared services, GPU load from other runs), or an acceptance bar stricter than the spec promises.
- Races and timing effects are fixed, in the test or in the code under test.
- An acceptance bar stricter than the spec promises is loosened to what the spec promises; a bar the spec does promise stays, and a real defect behind it is fixed.
- No test is dropped, skipped, tagged `@wip` or given a higher time limit to hide a failure.
- The stages `test`, `e2e-smoke`, `e2e-quick`, `e2e-seams`, `e2e-resilience`, `e2e-bench` and `refine-eval` pass on repeated runs of unchanged code.

Current stage: PoC. Target stage: PoC.
Scope: every Rust test and every Gherkin scenario that `just test`, `just check` and `just full-check` run; the test code, steps, harness and, where the cause is there, the app and command spotting code.
Acceptance bar: on one unchanged commit, `just full-check` passes 5 times in a row and `just test` 10 times in a row, with no rerun of any stage; the PR lists each failure cause found and its fix.
Out of scope: making the tests faster (sample sizes, parallel scenarios, duplicate tests, shorter timeouts) — a separate change; new behavior.

## Capabilities

### Modified Capabilities

- `voice-control`: scenarios that fail while the voice command handling is correct are made stable.
- `speech-recognition`: chunk seam scenarios that fail while transcription is correct are made stable.
- `text-refinement`: refinement evaluation that fails while refinement is correct is made stable.
- `local-services`: service start, outage and restart scenarios that fail while the services behave correctly are made stable.

## Impact

- `app/tests/`, `command-spotting/tests/`, `e2e-test/tests/features/*.feature` and `e2e-test/tests/steps/`, `.config/nextest.toml`, `justfile` recipes of the E2E stages; app or command spotting code where a race lives there.
- Commands: `just test`, `just check`, `just full-check`, `just e2e-*`, `just refine-eval`.

## Open points

- Which failures in past logs were real red tests during development (expected, TDD red) and which were flakes: proposed — count only failures on code that later passed unchanged, or that failed in one stage and passed on rerun.
- The 7 known-red seams rows moved to `@wip` by the audit of 2026-10-16: proposed — in scope; fix them or bring their bar to what the spec promises, and remove `@wip`.
- How many consecutive green runs prove stability: proposed 5 × `just full-check` and 10 × `just test`.
- Whether runs of other sessions beside the check (GPU load) count as a cause to fix: proposed — yes, a test must pass while another session's unlocked stages run beside it.
- Target version: proposed PATCH 0.18.1, assuming some fixes touch app or command spotting code; no release if only tests change.

## Release

SemVer: PATCH — fixes races and timing defects; no new behavior.
Target version: 0.18.1.
