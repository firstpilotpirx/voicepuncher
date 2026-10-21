## Why

The gate before every merge, `just full-check`, is a lottery: over 2026-10-20 and 2026-10-21 the stage journal (`~/.cache/voicepuncher/stage-times.jsonl`) holds 114 `full-check` runs and only 28 passed. The same tests fail on every branch, whatever its code changed. Each failure under the machine lock costs the change about an hour (≈ 45 min in the lock queue plus the run) and sends it back to the end of the queue, where 4–8 sessions wait; 34 of the 53 hours the lock queue took that day were spent on runs that then failed. Several optimization rounds of the checks did not help, because the failures are not slowness but nondeterminism.

Flaky under the machine lock, so not caused by parallel runs:

- E2E scenario "A hands-free session in a synthesized voice" — 17 failures (`e2e-smoke`)
- "The user's recorded starts give their cue at once" — 12 (`e2e-seams`)
- "Requests wait their turn when every copy is busy" — 9 (`e2e-smoke`)
- "The app refuses to start without the command spotting service" — 3
- "Restart said on its own gives its cue at once" — 2
- "Russian speech that makes speech recognition repeat itself" — 2

Flaky in `just test`, which runs without the lock beside other sessions' builds and tests:

- `voicepuncher-command-spotting::placed_commands_it` timing tests: `should_keep_a_start_confirmed_after_the_dictation_began_a_start` ×9, `should_keep_a_start_said_right_after_a_slowly_confirmed_stop_a_start` ×4, `should_report_a_restart_misheard_at_first_one_hearing_after_its_pause` ×4, `should_start_nothing_for_start_and_stop_with_a_dip_shorter_than_a_pause_in_standby` ×3, and four more of that file
- `voice_forms_it should_carry_out_every_recorded_way_of_saying_the_commands`
- `visualfeedback_macos_it`: `should_raise_waveform_columns_for_a_loud_level`, `should_show_the_record_sign_and_waveform_without_a_label`
- `commandspotting_websocket_it`: four tests (long listening with the phrase set switched, a command phrase inside a sentence, a 48 kHz microphone, several apps at once, a dictation opened by start)
- `transcription_openai_it`: `should_transcribe_corpus_sample_through_real_service` ×5, `should_give_up_a_hung_short_recording_after_three_attempts_within_their_waits` ×2

The pasteboard tests (`textpasting_macos_it`, 44 failures) were fixed by #253 (`clipboard-tests-machine-lock`) on 2026-10-21; the batch's `full-check` shows they stay green.

The user's words: flakes are evil; this change fixes them once and for all, not one at a time.

## What Changes

- Every test listed above that measures time with the wall clock while the machine is shared gets a fix in its harness (`placed_commands_it`, `commandspotting_reconnect_it`, `visualfeedback_macos_it`): it gives the same result on an idle and on a loaded machine.
- The E2E scenarios, the record-sign overlay test and the late-start tests are covered by six changes that landed meanwhile (`design.md` § Context); the tests that fail only against a real shared service (`voice_forms_it`, `commandspotting_websocket_it`, `transcription_openai_it`) and any E2E scenario still failing are measured by the batch's `full-check`, and each one still failing there gets a fix change of its own (`design.md` § Risks).
- No test is deleted and no bound is loosened without a measured reason stated in `design.md`.

Current stage: PoC. Target stage: PoC.
Scope: the tests and scenarios named above, their step code and harnesses (`app/tests/`, `command-spotting/tests/`, `e2e-test/tests/`), the nextest groups in `.config/nextest.toml`, and any product code a root cause points at.
Acceptance bar: the new busy-machine test fails on the wall-clock harness and passes on the speech-time one; `just test` passes, also with a real speech recognition service for commands; the batch's `full-check` shows which listed tests still fail, each then filed as a change of its own.
Out of scope: the lock queue itself (its wait limit, batch landing, how many sessions run at once) — that is a separate concern; new features; the speed of a passing check.

## Capabilities

### Modified Capabilities

- `voice-control`: the commands carried out depend only on what was said and how late each confirmation comes back in the speech, never on the machine's load.

## Impact

- `e2e-test/tests/features/voice_control.feature`, `local_services.feature` and their steps in `e2e-test/tests/steps/`
- `command-spotting/tests/placed_commands_it.rs`, `engine_it.rs`, and the engine's pause and confirmation code if a root cause is there
- `app/tests/voice_forms_it.rs`, `visualfeedback_macos_it.rs`, `commandspotting_websocket_it.rs`, `transcription_openai_it.rs`

## Open points

- Related changes: `steady-synthesized-hands-free-session`, `steady-overlay-waveform-test`, `ordered-wait-for-busy-copies`, `speed-up-short-start-cue`, `fast-confirmation-of-repeating-russian-speech` all landed on `main` before this change's design (`design.md` § Context); no dependency is left to declare.
- Whether the E2E flakes under the lock come from the user's own services sharing the GPU — settled: out of scope here, measured by the batch's `full-check` (`design.md` § Risks).
- Whether `placed_commands_it` needs the machine alone — settled: no; its confirmations run in speech time (`design.md` Decision 1).
- Target version: 0.32.1 (PATCH), the next above `main`.

## Release

SemVer: PATCH — bug fixes in tests and possibly in the product; no new capability.
Target version: 0.32.1.
