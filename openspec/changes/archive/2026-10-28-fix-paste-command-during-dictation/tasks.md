# Tasks

## 1. Paste is carried out during a dictation

- [x] 1.1 Red: rename `Rule: Each state accepts its own commands` to `Rule: Each state accepts its own commands, paste in both` in `app/tests/features/component/voice_control.feature` and `e2e-test/tests/features/voice_control.feature`
- [x] 1.2 Red: replace "Paste during a dictation is ignored" with "Paste during a dictation is carried out" in `app/tests/features/component/voice_control.feature`; run it, see it fail; commit it on its own as the intentionally red reproduction
- [x] 1.3 Red: add "Saying paste during a dictation pastes the clipboard and the dictation goes on" under `Rule: "paste text" pastes the clipboard` in the component feature, with its steps in `app/tests/steps/voice_control/`; run it, see it fail
- [x] 1.4 Green: add "paste text" to `DICTATING` in `voice-commands/src/lib.rs` and `VoiceCommand::DICTATING` in `app/src/core/domain/voice_command.rs`, with their list tests
- [x] 1.5 Run `just test` and commit

## 2. The command word stays out of the dictated text

- [x] 2.1 Red: unit tests in `app/src/core/domain/command_words/tests.rs` for removing up to n "paste" / "paste text" (with "please") that close a sentence, keeping one inside a sentence; run them, see them fail
- [x] 2.2 Green: the removal function in `app/src/core/domain/command_words/mod.rs`
- [x] 2.3 Red: add "A dictation with a paste in it pastes its text without the command" to the component feature, with its steps; run it, see it fail
- [x] 2.4 Green: count the pastes in `CommandWords` and remove them in `Dictation::without_command_words` (`app/src/core/application/pipeline/dictation.rs`, `voice_control/mod.rs`)
- [x] 2.5 Run `just test` and commit

## 3. Version and manual checks

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.41.4
- [x] 3.2 Run `just test` and commit
- [x] 3.3 Manual check: with a dictation running, copy a text, say "paste" — the text appears at the cursor, the dictation keeps recording and later pastes its own text without "paste" (skipped by the user)
