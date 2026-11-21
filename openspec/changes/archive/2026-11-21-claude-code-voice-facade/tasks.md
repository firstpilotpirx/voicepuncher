## 1. Names, shortcut and configuration

- [x] 1.1 Add "Claude dictation" and "KeyChord" to `docs/ubiquitous-language.md`; note under "Key press" that "shortcut" names only another app's own KeyChord (the Claude dictation shortcut)
- [x] 1.2 Red: unit tests in `app/src/core/domain/key_chord.rs`: `cmd+d` and `cmd+5` parse (Command plus the key), `d`, `ctrl+d`, `cmd+dd`, `cmd+` and an empty text are refused naming the text; run with `just test-one`, see them fail
- [x] 1.3 Green: `KeyChord` in `app/src/core/domain/key_chord.rs`, exported from `core/domain`
- [x] 1.4 Red: config tests (`app/src/config/claude_dictation/tests.rs`): an empty file gives `[claude_dictation]` `enabled = true`, `shortcut = "cmd+d"`; `shortcut = "d"` fails to load naming `claude_dictation.shortcut`; run with `just test-one`, see them fail
- [x] 1.5 Green: `ClaudeDictationCfg` in `app/src/config/mod.rs`, checked in `validate`; the `[claude_dictation]` block with its comments in `app/src/config/default.toml`
- [x] 1.6 Red: settings window tests: every option has a control (fails on the two new options), and the shortcut field refuses `d`; run with `just test-one`, see them fail
- [x] 1.7 Green: the two rows on the Dictation tab (`app/src/adapters/primary/settings/macos/table/mod.rs`), the switch and, under the advanced disclosure, a `Control::Shortcut` drawn as a text field and validated with `KeyChord::parse`, as the agreed mock shows
- [x] 1.8 `just test`, commit

## 2. The pipeline drives Claude's dictation

- [x] 2.1 Port `ClaudeDictation` in `app/src/core/application/claude_dictation.rs` (`is_frontmost`, `toggle`, `cancel`), mockable, exported from `core/application`
- [x] 2.2 Red: in `app/tests/features/component/voice_control.feature`, the rule "In Claude, a voice start starts Claude's own dictation" (two scenarios), the rule "In Claude, voice commands end Claude's own dictation" (four scenarios) and the rule "The Claude dictation falls back to the app's own dictation" with the turned-off and the failed-press scenarios; the start scenario also checks the `voice start: total=…ms` log line; steps in `app/tests/steps/voice_control/claude.rs` with a stand-in `ClaudeDictation` that records its presses and can be made to fail; run with `just test-one 'scenario: <name>'`, see them fail
- [x] 2.3 Red: in `app/tests/features/component/push_to_talk.feature`, the rule "In Claude, the hotkey drives Claude's own dictation" with the Toggle and Hold scenarios; steps beside the voice ones in `app/tests/steps/voice_control/claude.rs`, plus a scenario that the user's own Escape presses no second Escape; see them fail
- [x] 2.4 Green: `app/src/core/application/pipeline/claude_dictation.rs` and `Pipeline::with_claude_dictation`: a start asks the port, presses the shortcut first and only then plays the cue and shows the overlay through `show_recording_then`, so `voice start: total=…ms` is still logged; stop presses it again, plays the stop sound and hides the overlay; `abandon` presses Escape; `close_dictation` cuts the recorder only for the app's own dictation; "press enter" waits `CLAUDE_TEXT_WAIT` (1.5 s) before Return; a failed press logs `claude dictation: not used: <why>` and starts the app's own dictation; `is_recording` counts a Claude dictation
- [x] 2.5 Refactor within the size budgets; `just test`, commit

## 3. macOS adapter and wiring

- [x] 3.1 Red: unit tests in `app/src/adapters/secondary/claudedictation/macos/mod.rs`: a `KeyChord` maps to its key code with the Command flag; Escape is pressed with no modifier; an untrusted process fails with `AccessibilityMissing`; the bundle id `com.anthropic.claudefordesktop` counts as Claude and another does not; see them fail
- [x] 3.2 Green: `MacOsClaudeDictation`, reading `NSWorkspace.frontmostApplication.bundleIdentifier` (the `NSWorkspace` and `NSRunningApplication` features `objc2-app-kit` already has) and pressing through `keystroke::press` at keyboard level
- [x] 3.3 Wire it in `app/src/runtime/wiring.rs` when `claude_dictation.enabled`, logging `claude dictation: on (cmd+d)` or `claude dictation: off`
- [x] 3.4 `just test`, commit

## 4. Docs, version and checks

- [x] 4.1 README: a "Claude dictation" section (what it does, the setting, the known limit that a spoken "stop dictation" stays in Claude's text) and a release checklist line for it; the adapter in `docs/architecture/project-architecture.html` where the secondary adapters are listed and in `docs/project-map.md`; the offline wording of `docs/architecture/overview.md` ("Nothing leaves the machine", "Fully offline") and `AGENTS.md` § Key constraints: voicepuncher sends nothing off the machine; in Claude, Claude's own dictation records by the user's choice
- [x] 4.2 Bump `version` in `app/Cargo.toml` to 0.50.0
- [x] 4.3 `just test`, commit; `just check` once
- [x] 4.4 Manual run: in the Claude app, "start", a sentence, "stop" puts Claude's own text in its composer; the hotkey does the same; in TextEdit dictation records and pastes as before — skipped by the user on 2026-11-21
