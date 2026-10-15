# Tasks

## 1. Pipeline sends a signal per processing stage

- [x] 1.1 Red: add `app/tests/features/component/processing_stages.feature` (`@processing_stages @component`) with the seven component scenarios of design § Test levels, and steps in `app/tests/steps/processing_stages.rs` that drive the real `Pipeline` with doubles logging overlay signals and the start and end of transcription and refinement (whole recording and live chunks, text refinement on and off, a focus probe with no text input, voice control); run them and see them fail because no stage signal exists
- [x] 1.2 Green: replace `VisualIndicator::show_processing` with `show_recognizing_speech` and `show_refining_text`; the pipeline sends `show_recognizing_speech` at the stop and `show_refining_text` before refinement unless `without_text_refinement_stage()` was called; `main.rs` calls it with `--no-llm`; update every implementation and test double (`NoopVisualIndicator`, the macOS overlay with the spinner for both until group 2, `app/tests/steps/*`, `e2e-test/tests/steps/*`) and the pipeline unit tests; the voice-control step "the overlay shows processing…" accepts the speech recognition stage as the start of processing; `just test` is green
- [x] 1.3 `just check` and commit

## 2. The macOS overlay shows an icon per stage

- [x] 2.1 Red: unit test in the macOS overlay that the speech recognition and text refinement icons differ and neither is the recording or focus sign; run it and see it fail (`visualfeedback_macos_it.rs` only follows the rename: off the main thread its tests return before asserting)
- [x] 2.2 Green: `OverlayState::RecognizingSpeech` and `OverlayState::RefiningText` instead of `Processing`; each shows its icon in the left label with the spinner turning; a screen change redraws the current stage
- [x] 2.3 Add the **Processing stage** term and the new signals of `VisualIndicator` to `docs/ubiquitous-language.md`
- [x] 2.4 `just check` and commit

## 3. Release

- [x] 3.1 Bump `version` in `app/Cargo.toml` to the next MINOR version (0.14.0 unless `main` moved on), `just check` and commit
- [x] 3.2 Manual run by the user (overlay): `cargo run --release -p voicepuncher` with and without `--no-llm`; dictate by hotkey and by voice; the overlay shows 👂 after the stop, then ✏️ (only with refinement), then hides at the paste; a short and a long dictation (live chunks); stop→paste time as before
