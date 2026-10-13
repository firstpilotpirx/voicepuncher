# Tasks

## 1. Matcher and recognizer

- [x] 1.1 Red: unit tests for `PhraseMatcher` in `command-spotting/src/matcher.rs` — exact phrases, trailing "please", aliases, repeats, the user's slips ("Star dictation", "Start. Activation.", "Cancel the quotation please", "Clear all texts"), and non-commands ("please press enter", "press enter when you are ready", "start", "hello") → none; see them fail
- [x] 1.2 Green: implement `PhraseMatcher`; `just test` passes
- [x] 1.3 Red: recognizer unit tests — two phrases of the same command in one segment yield one command; of different commands reported before the segment closes none; a different phrase after the first was carried out is ignored; see them fail
- [x] 1.4 Green: implement; `just test` passes
- [x] 1.5 `just check` and commit

## 2. Service: keywords and whisper confirmation

- [x] 2.1 Add the new keyword lines; engine_it spots "press enter please" and "clear text" from `say`
- [x] 2.2 Red: user-voice adapter integration test `app/tests/voice_samples_it.rs` streaming the session recording; see it fail (24/40)
- [x] 2.3 Green: pre-roll ring buffer, whisper client, confirmation of short segments, `--transcription-url` / `--max-command-secs`; justfile passes the transcription URL to the service and exports `VOICEPUNCHER_VOICE_SAMPLES`; the test passes
- [x] 2.4 E2E scenarios for "press enter please", "stop dictation please", "clear text" in `e2e-test/tests/features/voice_control.feature`; `just e2e-smoke` passes; corpus scenario still finds no command
- [x] 2.5 README table and glossary; `just check` and commit

## 3. Whole sessions and review fixes

- [x] 3.1 Switch the phrase set to the dictation phrases before the recording starts (pipeline unit test first)
- [x] 3.2 E2E hands-free sessions: synthesized voice, the user's recorded commands, and long pauses between steps
- [x] 3.3 `voice_samples_it` counts exactly one command per utterance (repeats under 0.5 s are one)
- [x] 3.4 `engine_it`: "press enter please", "stop dictation please", "clear text", "clear text please" by keyword alone; keyword spotter alone when whisper does not answer
- [x] 3.5 E2E: short Russian phrases give no command
- [x] 3.6 `just check` and commit

## 4. Release and manual checks

- [x] 4.1 Bump `version` in `app/Cargo.toml` to 0.8.0; `just check` and commit
- [ ] 4.2 Manual run by the user: each command, with and without "please", "clear text"; the log shows each command
