## 1. Press keys as a keyboard does (D1)

- [x] 1.1 Red: add the rule "Keys pressed in the focused app arrive as a keyboard's" with its two scenarios to `app/tests/features/slice/text_insertion.feature`, steps in `app/tests/steps/text_insertion/keystrokes.rs`, against a keystroke function that posts as today; run with `just test-one` and see it fail
- [x] 1.2 Commit the failing test (`test: reproduce key presses with no pause and no Command key`)
- [x] 1.3 Green: `app/src/adapters/secondary/keypressing/macos/keystroke.rs` posts modifier keys of their own and a 10 ms pause after every event; `MacOsKeyPresser::press` uses it
- [x] 1.4 Green: `MacOsPaster::post_cmd_v` in `app/src/adapters/secondary/textpasting/macos/mod.rs` uses it
- [x] 1.5 Describe the keyboard-like keystroke in `docs/architecture/context.md` (the Cmd+V line) and `docs/ubiquitous-language.md` (the `CGEventPost` line)
- [x] 1.6 `just test`, commit

## 2. Release

- [x] 2.0 Post the voice commands' keys at the HID level and a dictation's paste at the session level, and hold only Command as a key of its own, after the manual run

- [x] 2.1 Bump `version` in `app/Cargo.toml` to 0.48.1; `just check` once
- [ ] 2.2 Manual run: the user says "paste text" in Ghostty and sees the BetterTouchTool Cmd+V trigger fire; a hotkey dictation still pastes
