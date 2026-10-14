# Tasks

## 1. Reproduce the bug

- [x] 1.1 Red: component `.feature` scenarios in `app/tests/features/component/voice_control.feature`: "start start" reported only after the sentence said right after it began; "start", a 0.5 s pause and a sentence reported late; "restart", a pause and another sentence; "stop enter start" and another sentence. Step code makes the scripted service report each command late. Run `just test` and see each fail because the sentence's first words are missing; commit it alone as `test: reproduce speech lost after a voice start` (intentionally red)

## 2. Every voice start begins at the command's end

- [x] 2.1 Red: pipeline unit tests in `voice_control.rs`: every placement of "start" and "restart" asks for the sound heard since the command's end; the recorder starts before the heard sound is taken and the join comes before live chunks start. See them fail
- [x] 2.2 Green: `start_recording_from(Option<Instant>)`; `AudioRecorder::begin_with` replaces `start_with_lead_in` (cpal adapter, component and E2E stand-ins); `opening` for every voice start not said alone. Verify with `just test`, component scenarios of group 1 green
- [x] 2.3 Red: `command_words` unit test for a doubled leading "start start" removed from the text; see it fail
- [x] 2.4 Green: `without_leading_start` removes a doubled form; `just test`
- [x] 2.5 Hold the heard sound for 10 s (`HELD_SECS`), with its unit test updated; `just check` and commit
- [x] 2.6 Red: `command_words` unit tests for a leading "restart", "restart restart" or "restart dictation" removed from a restarted dictation's text, and kept after another command; a pipeline unit test pasting a restarted dictation whose text opens with "restart"; the restart scenarios' Then rejects command words; see them fail
- [x] 2.7 Green: `without_leading_command(command, text)` replaces `without_leading_start`, with the forms of the command that started the dictation; `opening` holds that command; `just test`

## 3. End to end, synthesized and the user's voice

- [x] 3.1 Red: E2E `.feature` scenarios in `e2e-test/tests/features/voice_control.feature`: "start start" and a sentence in one breath; "start", 0.5 s, a sentence; "restart", 0.5 s, another sentence (synthesized, streamed in real time); the user's recorded "start start" from `session-2` and "restart restart" from `session-5`, each followed at once by a sentence (`@voice_samples`). The hands-free harness's spotter hands out the adapter's heard sound and its recorder puts the lead-in first. Each asserts every word of the sentence is pasted and no word of "start", "restart" or "dictation" is. See them fail on the code before group 2 (temporarily revert the pipeline change locally), then pass; `just check` and commit

## 4. Docs and release

- [x] 4.1 README voice control section: speech right after any voice start is kept. Glossary (`docs/ubiquitous-language.md`): no new term; check that the *command placement* entry still holds. Commit
- [x] 4.2 Bump `version` in `app/Cargo.toml` to 0.10.1 (or the next patch if `main` moved on); `just check` and commit
- [ ] 4.3 Manual run by the user (stage 10, after the PR): "start start" and speech at once, "start", a short pause and speech, "restart", a short pause and speech; the pasted text keeps the first words

## Workflow follow-up

- Verify (`opsx:verify`, `spec-test-reviewer`), archive, `just full-check`, PR, manual run, merge, release 0.10.1.
