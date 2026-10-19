# Tasks

## 1. Level waveform model

- [x] 1.1 Red: unit tests in `app/src/core/domain/level_history.rs` — speech gives tall columns, silence 2 pt dots, a new history is all dots, 30 columns keep the last 3 s, rise/fall smoothing, slide offset per 100 ms; see them fail
- [x] 1.2 Green: implement `LevelHistory` (design D6), remove `level_to_pills` and its tests
- [x] 1.3 Update `docs/ubiquitous-language.md`: `AudioLevel` drives the level waveform; add "Level waveform"
- [x] 1.4 `just test`, commit

## 2. Pasted sign and refinement preview in the core

- [x] 2.1 Red: `app/tests/features/component/visual_feedback.feature` with the Pasted scenarios (pasted, pasted after a wait, none for the clipboard, replaced by a command) and steps in `app/tests/steps/visual_feedback.rs`; see them fail
- [x] 2.2 Red: update `processing_stages.feature` (Pasted after the paste, dimmed raw transcript during refinement), `text_insertion.feature` (Pasted after the wait) and `voice_control.feature` (hotkey dictation ends on Pasted) with their steps; see them fail
- [x] 2.3 Green: `VisualIndicator::show_pasted(text)` and `show_refining_text(raw)` (design D1); the pipeline shows Pasted after `Pasted` / `PastedAfterWait`, a pending notice still wins; update mocks, `NoopVisualIndicator` and unit tests in `pipeline.rs` and `text_insertion.rs`
- [x] 2.4 Add "Pasted sign" to `docs/ubiquitous-language.md` and update "Processing stage" (icons, labels, dimmed raw transcript)
- [x] 2.5 `just test`, commit

## 3. Pointer badge

- [x] 3.1 Red: rewrite `app/tests/features/component/dictation_pointer.feature` for the new rules (mirrors every stage, failure triangle, command sign leaves it, goes with the overlay, none for CMD alone) and its steps; unit tests of the badge mapping in `pointer_sign.rs`; see them fail
- [x] 3.2 Green: badge state in `PointerSignView::show`, `PointerSignIndicator` maps every signal, Pasted and failure badges hide after 0.5 s and 4 s (design D3)
- [x] 3.3 Draw the badge in `macos/pointer_sign.rs`: 38 pt dark-glass circle, rim, SF Symbol per state, turning activity ring while processing; keep the position; `visualfeedback_macos_it` badge tests pass
- [x] 3.4 Update "Dictation sign" in `docs/ubiquitous-language.md`
- [x] 3.5 `just test`, commit

## 4. Glass overlay

- [x] 4.1 Red: unit tests in `macos/mod.rs` for state → SF Symbol, English label and tint (Recognizing, Refining text, Click a text field, Pasted, Enter, Clear, Restart, CMD, failure short names), replacing the glyph tests; see them fail
- [x] 4.2 Green: the mapping functions (design D7)
- [x] 4.3 Glass pill and text window: `NSGlassEffectView` on macOS 26, `NSVisualEffectView` fallback, geometry, rim and shadow of `mockups/design-spec.md` § 2; Cargo features; `visualfeedback_macos_it` passes, the glass takes no focus
- [x] 4.4 Icons, breathing, record dot pulse, spoke spinners and Reduce Motion (design D5)
- [x] 4.5 Waveform layer driven by `LevelHistory` on the main-queue tick, left fade mask, 22 columns plus separator and `mic.slash` when commands are unavailable
- [x] 4.6 Waiting, Pasted (0.5 s then hide), command, failure (dark triangle over light), CMD and dimmed refinement preview states; adapter test for the Pasted hide in `visualfeedback_macos_it.rs`
- [x] 4.7 `just test`, `just check`, commit

## 5. Release

- [x] 5.1 Bump `version` in `app/Cargo.toml` to 0.24.0 (0.23.0 went to add-voice-command-paste); `just test`, commit
- [x] 5.2 Manual run by the user: every state of `mockups/design-spec.md` § 8 over a blue, a light and a dark window, with and without `--no-llm`, Reduce Motion on once; the user pastes a healthy log
