# Tasks

## 1. Reproduce the dropped phrase (Red)

- [x] 1.1 Red: add the E2E scenario "English speech with a short pause after every word keeps every phrase" to `e2e-test/tests/features/speech_recognition.feature` (`@seams`, lead-ins 0, 0.7 and 1.4 s) with a lead-in step in `e2e-test/tests/steps/speech_recognition.rs`; verify it fails on the unchanged code under `just e2e-seams` with a dropped phrase of 26 words, and commit it alone as `test: reproduce dropped phrase at word pauses` (intentionally red)

## 2. Keep the speech, remove the hallucination (Green)

- [x] 2.1 Red: add unit tests in `app/src/core/application/transcript_filter.rs` for `strip_whisper_hallucinations` — speech followed by "Thanks for watching." keeps the speech, a credit with a name is removed with its sentence, a marker inside speech is removed, hallucination-only inputs (credits, markers, "you", loops) give no text, speech without hallucination is unchanged — and adapter integration tests in `app/tests/transcription_openai_it.rs` (stub answers speech plus "Thanks for watching.", speech with "[BLANK_AUDIO]" inside); verify `just test` fails only on them, and commit them as intentionally red
- [x] 2.2 Green: implement `strip_whisper_hallucinations` (markers removed as tokens, phrases remove their sentence, the remainder dropped when empty, a loop or only "you") and remove `is_whisper_hallucination`, moving its tests to the new function; verify the unit tests pass with `just test`
- [x] 2.3 Green: call `strip_whisper_hallucinations` in the speech recognition adapter, logging removed hallucinations and dropped transcripts; verify the adapter integration tests and `just test` pass
- [x] 2.4 Verify `just check` passes, and under `just e2e-seams` the seam sweep (`chunked_transcription_it`, all 30 conditions) and the new E2E scenario pass with no dropped phrase; the 7 "Long speech under difficult recording conditions" scenarios that `e2e-seams` reaches for the first time fail on misheard words, not on dropped phrases, and go to the change `raise-accuracy-under-hard-conditions`

## 3. Release

- [x] 3.1 Bump `version` in `app/Cargo.toml` to the next PATCH on top of `origin/main` (0.13.1 unless `main` moved on); verify `cargo metadata` shows it and `just test` passes

## Workflow follow-up

- `/opsx:verify`, `spec-test-reviewer`, `/opsx:archive`, `just full-check` (corpus mean WER on 100 recordings no worse than on `main`), PR, merge, `just release`.
