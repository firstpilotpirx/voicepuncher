# Tasks

## 1. Matcher and text strip

- [x] 1.1 Red: matcher unit tests for `at_end`, `start_at_head`, `anywhere` (doubled forms and chains) (positive, please, fuzzy, and negatives: command word inside a sentence, enter followed by speech, pairs the states do not allow); see them fail
- [x] 1.2 Green: implement; `just test`
- [x] 1.3 Red: domain unit tests for stripping command words from dictated text (tail, head, from a doubled or chained command on); see them fail
- [x] 1.4 Green: implement; `just check` and commit

## 2. Service

- [x] 2.1 Red: `placed_commands_it` with `say` and a transcription service: stop at the end of an utterance, start at its head, stop stop and stop enter in the middle, stop after a 0.35 s pause, and no command for a command word inside a sentence; see them fail
- [x] 2.2 Green: `Command` event, phrase selection with the next state's phrases, closing-utterance tail check, running-utterance head and sliding windows, whisper segment timestamps, `--min-silence-secs` 0.3; `just test`
- [x] 2.3 `just check` and commit

## 3. App

- [x] 3.1 Red: recognizer unit tests for `Command` events (one, two in order, not repeated); pipeline unit tests for the head-start lead-in, enter during a dictation and restart; component `.feature` scenarios of the spec; see them fail
- [x] 3.2 Green: protocol, recognizer, pipeline, `CommandSpotter::heard_since`, recorder lead-in; `just test`
- [x] 3.3 E2E `.feature` scenarios with `say` for every spec scenario; corpus and short Russian phrases still give no command; `just check` and commit

## 4. Restart, cancel alone and the user's voice

- [x] 4.0 Restart: matcher, keywords, app command, component and E2E scenarios; "cancel" alone: matcher and E2E

- [x] 4.1 The user records `session-2` (every form) and `session-3` ("cancel"); cut and label them into `forms.jsonl`; `voice_forms_it` passes on them and `voice_samples_it` on `session-1`

## 5. Docs and release

- [x] 5.1 README voice control section, glossary
- [x] 5.2 Bump `version` in `app/Cargo.toml` to 0.10.0; `just check` and commit
- [ ] 5.3 Manual run by the user (stage 10, after the PR)
