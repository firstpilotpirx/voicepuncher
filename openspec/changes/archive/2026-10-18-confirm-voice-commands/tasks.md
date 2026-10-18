# Tasks

## 1. Sounds per command

- [x] 1.1 Red: in `app/src/config.rs` tests, assert the defaults `enter_sound` Morse, `clear_sound` Purr, `restart_sound` Hero, all distinct from the other sounds, and that `command_sound` in a config file sets `enter_sound`; `just test` fails
- [x] 1.2 Green: add the three fields with the `command_sound` alias to `SoundsCfg`, replace `play_command` with `play_enter`, `play_clear`, `play_restart` in `SoundPlayer` and `MacosSoundPlayer`, wire them in `app/src/main.rs`; `just test` passes
- [x] 1.3 `just test`, commit (`feat(voice-control): a sound per confirmed command`)

## 2. Command sign in the core

- [x] 2.1 Red: in `app/tests/features/component/voice_control.feature` change the press enter, clear all text, enter during a dictation and restart scenarios to the new sounds and signs, add the rules "Every confirmed command sounds different" (enter vs clear), "A confirmation sign gives way to what comes next" (enter start; stop clear enter) and "Push-to-talk keeps its sounds and overlay", plus steps in `app/tests/steps/voice_control.rs`; run them and see them fail
- [x] 2.2 Green: add `VisualIndicator::show_command_sign(VoiceCommand)`; call it with `play_enter` / `play_clear` after the key press in `pipeline/voice_control.rs`
- [x] 2.3 Green: split `cancel_recording` and `start_recording_from` in `pipeline.rs` so restart discards without the cancel cue, starts with `play_restart`, shows the recording, then the restart sign; update the unit test `should_play_the_cancel_sound_then_the_start_sound_on_restart`; `just test` passes
- [x] 2.4 `just test`, commit (`feat(voice-control): show a sign for enter, clear and restart`)

## 3. Command sign on the macOS overlay

- [x] 3.1 Red: unit tests in `app/src/adapters/secondary/visualfeedback/macos/mod.rs` for the sign text of each command (none for start, stop, cancel) and the state the overlay returns to after the sign (hidden from hidden, recording from recording); `just test` fails
- [x] 3.2 Green: show the sign in place of the state's sign and schedule the return after about 1 s, cancelled by the generation counter on any later signal; `just test` passes
- [x] 3.3 `just test`, commit (`feat(overlay): command sign`)

## 4. Glossary and docs

- [x] 4.1 Update `docs/ubiquitous-language.md`: `VisualIndicator` six signals, `SoundPlayer` per-command sounds, new term *Command sign*
- [x] 4.2 Update the README sounds section with the three command sounds
- [x] 4.3 `just test`, `just check`, commit (`docs: command sign and sounds`)

## 5. Version and manual run

- [x] 5.1 Bump `version` in `app/Cargo.toml` to 0.18.0; `just test`; commit (`chore: bump version to 0.18.0`)
- [ ] 5.2 Manual run by the user: "enter", "clear", "restart", "enter start" and a hotkey dictation show and sound as the spec says; with `clear_sound` set to another system sound in the config, "clear all text" plays that sound
