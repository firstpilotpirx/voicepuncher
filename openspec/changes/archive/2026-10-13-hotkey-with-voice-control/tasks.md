# Tasks

## 1. Overlay and sound of a voice dictation

- [x] 1.1 Debug logs at the overlay and sound adapters; manual run by the user with a voice dictation: overlay and start sound present in 0.8.0
- [x] 1.2 Component scenarios "Saying start dictation starts a dictation" (overlay) and "Saying stop dictation plays the stop sound and shows processing", with a recording overlay stand-in
- [x] 1.3 Logs at debug level; `just test` passes

## 2. Hotkey records with voice control on

- [x] 2.1 Red: component scenarios for the hotkey with voice control on (records, ends its dictation, Escape cancels, ends a voice dictation, listening stays on, voice stop and cancel end a hotkey dictation), the scripted service reporting only the phrases it listens for; pipeline unit tests for hotkey recording and hold release; listening-toggle scenarios and tests removed; seen failing
- [x] 2.2 Green: hotkey signals through `step`; phrase set switched in `start_recording`/`stop_recording`/`cancel_recording`; `toggle_listening` removed; `just test` passes
- [x] 2.3 Commit

## 3. Short forms

- [x] 3.1 Red: matcher unit tests for "enter", "clear", "start", "stop" with and without "please", and other short words as none; seen failing
- [x] 3.2 Green: matcher aliases; E2E short-form scenarios; "стоп" out of the short Russian phrases; `voice_samples_it` still passes
- [x] 3.3 README, glossary, CLI help

## 4. Release and manual checks

- [x] 4.1 Bump `version` in `app/Cargo.toml` to 0.9.0; `just check` and commit
- [ ] 4.2 Manual run by the user: hotkey dictation and voice dictation with `--voice-control`, each ended by the other; overlay and sounds every time; the short forms
