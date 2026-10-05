# Tasks

## 1. Paste-first rule (domain)

- [x] 1.1 Red: unit tests in `app/src/core/domain/text_focus.rs` — text roles, `AXWebArea`/`AXGroup`, no element, AX error, listed role with a text selection or a settable value → paste; each listed non-text role with no flags → wait; the decision's description names the role and flags; run `just test` and see them fail
- [x] 1.2 Green: add `FocusedElement` and `TextFocus::of` with the fixed non-text role list; `just test` passes
- [x] 1.3 Add *text input focus* and *focus wait* to `docs/ubiquitous-language.md`
- [x] 1.4 `just check` and commit

## 2. Ports, setting and insertion core

- [x] 2.1 Red: unit test that `[paste]` without `focus_wait_seconds` loads 300 and an explicit value is kept (`app/src/config.rs`); see it fail
- [x] 2.2 Green: add `focus_wait_seconds` to `PasteCfg`
- [x] 2.3 Add port `FocusProbe` (`app/src/core/application/focus_probe.rs`), `TextPaster::leave_in_clipboard`, `VisualIndicator::show_waiting_for_focus`; implement them in the existing adapters (macOS paster: set clipboard without Cmd+V or restore; stdio: print; macOS overlay: stub calling `show_processing` until group 4)
- [x] 2.4 Red: unit tests for `insert_text` in `app/src/core/application/text_insertion.rs` with mocks — paste at once without waiting, wait then paste when the probe flips, timeout → `leave_in_clipboard`, `Pressed` / `Cancel` → `leave_in_clipboard`, `Released` ignored, zero timeout or no probe → paste with no probe call, waiting indicator shown once; see them fail
- [x] 2.5 Green: implement `insert_text`, `FocusWaitCfg` and the log lines of design D8; `just test` passes
- [x] 2.6 `just check` and commit

## 3. Pipeline wiring and slice feature

- [x] 3.1 Red: write `app/tests/features/slice/text_insertion.feature` (`@text_insertion @slice`, one `Rule:` per requirement except the config-default and log scenarios that are unit-tested) and its steps in `app/tests/steps/text_insertion.rs` with a scripted focus probe, recording paster and indicator driving the real `Pipeline`; run `just test` and see it fail
- [x] 3.2 Green: `Pipeline` keeps the signal channel (`run` stores it), takes the probe and `FocusWaitCfg` through `with_focus_wait`, and both dictation paths call `insert_text`; `just test` passes
- [x] 3.3 `just check` and commit

## 4. macOS adapters

- [x] 4.1 Add `MacOsFocusProbe` in `app/src/adapters/secondary/focus/macos/` (AX FFI, 0.2 s messaging timeout); integration test `app/tests/focus_macos_it.rs` gated by `VOICEPUNCHER_REAL_AX_IT` asserting the call returns without panic and reports a role when an element is focused
- [x] 4.2 Overlay `WaitingForFocus` state (`⌨ FOCUS`, orange, pulsing) in `adapters/secondary/visualfeedback/macos`, re-applied on screen rebuild
- [x] 4.3 Wire the probe and `FocusWaitCfg` (timeout from config, 100 ms poll) in `app/src/main.rs` for dictation mode only; architecture test still passes
- [x] 4.4 Document `focus_wait_seconds` in `app/config.example.toml` (`[paste]`) and in the setup wizard if it writes `[paste]`
- [x] 4.5 `just check` and commit

## 5. Release and manual checks

- [x] 5.1 Bump `version` in `app/Cargo.toml` to 0.5.0; `just check` and commit
- [x] 5.2 Manual run by the user (`cargo run --release -p voicepuncher`, after `just start-model-servers`): dictate into a text field (paste at once); click the Finder sidebar/a button before the dictation ends, then click into a text field (overlay `⌨ FOCUS`, then paste); let a wait end by the hotkey (text in clipboard, no recording) and press again (recording); dictate into Terminal, a browser text field and an Electron app (no wait); the log shows each `focus:` decision
