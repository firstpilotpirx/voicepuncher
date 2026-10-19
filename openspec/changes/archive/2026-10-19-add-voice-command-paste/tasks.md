## 1. Command spotting service hears "paste text"

- [x] 1.1 Red: unit tests in `command-spotting/src/keywords.rs` (seven phrases), `matcher.rs` ("Paste." and "Paste please." → "paste text") and `matcher/placed.rs` ("paste paste" doubled, "paste enter" and "stop paste enter" chains, "paste" refused during a dictation); run, see them fail
- [x] 1.2 Green: add the "PASTE TEXT" lines to `command-spotting/keywords.txt`, the "paste" alias to `matcher.rs`, the forms and standby phrase to `matcher/placed.rs`
- [x] 1.3 `just test`, commit

## 2. The app carries out "paste text"

- [x] 2.1 Red: unit tests for `VoiceCommand::PasteText` (phrase, standby list) in `app/src/core/domain/voice_command.rs`, Cmd+V and the untrusted error in `app/src/adapters/secondary/keypressing/macos/mod.rs`, the `Frog` default and `paste_sound` override in `app/src/config.rs`, the `⎘ PASTE` sign in the overlay sign table; run, see them fail
- [x] 2.2 Red: component scenarios in `app/tests/features/component/voice_control.feature` — paste text presses Cmd+V with the paste sound and sign, the clipboard is never written, paste during a dictation is ignored, "paste enter" presses Cmd+V then Return, paste sounds different from enter and clear — with steps and doubles in `app/tests/steps/voice_control.rs` and `doubles.rs`; run, see them fail
- [x] 2.3 Green: add `PasteText` to `VoiceCommand`, `paste()` to `KeyPresser` and `MacOsKeyPresser`, `play_paste` to `SoundPlayer` and its macOS adapter, `paste_sound` to `SoundsCfg` and `app/config.example.toml`, the sign to the overlay, the command to `press_keys` and `confirm` in `pipeline/voice_control.rs`, the failure notice text
- [x] 2.4 `just test`, commit

## 3. End to end and docs

- [x] 3.1 Red: E2E rows in `e2e-test/tests/features/voice_control.feature` — "paste text", "paste", "paste please" alone, "Okay, paste" at the end, "Fine, paste paste", "Okay, paste, enter" chain, "I will paste the link later" ignored — and the "paste" short form in `e2e-test/tests/steps/voice_control.rs`
- [x] 3.2 Green: whatever the E2E rows need beyond groups 1–2 (keyword thresholds in `keywords.txt`)
- [x] 3.3 Add "paste text" to `docs/ubiquitous-language.md` (variants, key press, sound player) and the README command tables, chains, sounds and release checklist
- [x] 3.4 `just test`, commit

## 4. Version and manual checks

- [x] 4.1 Bump `version` in `app/Cargo.toml` to 0.23.0
- [x] 4.2 `just test`, `just check`, commit
- [x] 4.3 Manual run by the user: copy a sentence, focus a chat input, say "paste" → the sentence is pasted with the paste sound and the `⎘ PASTE` sign; say "paste enter" → pasted and sent; the clipboard still holds the sentence
