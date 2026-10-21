# Tasks

## 1. Capsule dictation cancels the dictation (design decision 1)

- [x] 1.1 Red: unit test in `command-spotting/src/matcher.rs` — while dictating, "Capsule dictation" is matched as "cancel dictation"; run it, see it fail, commit it alone (`test: reproduce …`, intentionally red).
- [x] 1.2 Green: add `("capsule dictation", "cancel dictation")` to `ALIASES`.
- [x] 1.3 `just test`; commit.

## 2. Please after a voice start is not pasted (design decision 2)

- [x] 2.1 Red: component scenario "Please after start dictation is not pasted" in `app/tests/features/component/voice_control.feature` with its step in `app/tests/steps/voice_control/speech_after_start.rs`, and a unit test in `app/src/core/domain/command_words.rs`; run them, see them fail, commit them alone (`test: reproduce …`, intentionally red).
- [x] 2.2 Green: `without_leading_command` drops a "please" that opens the text after a voice start when more text follows.
- [x] 2.3 `just test`; `just check` once; commit.

## 3. Version

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.31.1. Manual checks: none (no hotkey, recording, overlay, paste or permission change).
