## 1. Glass edge and transparency

- [x] 1.1 Red: `visualfeedback_macos_it` test — the pill and text window glass have no top-highlight layer and, on macOS 26, use the clear system glass style; run it, see it fail; commit as intentionally red
- [x] 1.2 Green: drop the highlight layer, drop our rim on the system glass, set the clear style in `macos/glass.rs`
- [x] 1.3 `just test`; commit

## 2. Text window placement

- [x] 2.1 Red: `visualfeedback_macos_it` test — with a live transcript shown, the text window's real frame lies below the pill's real frame, not overlapping; log both frames; run it, see it fail; commit as intentionally red
- [x] 2.2 Green: no window margin above the pill glass in `macos/glass.rs` / `macos/window_set.rs`; place the text window from the pill's real glass frame in `macos/transcript_window.rs`
- [x] 2.3 `just test`; commit

## 3. Text window only while recording

- [x] 3.1 Red: `visualfeedback_macos_it` tests — no text window for refinement, wait for focus, Pasted and a failure notice; the text window hides when the recording ends; replace the dimmed-preview and wait-explanation tests; run, see them fail; commit as intentionally red
- [x] 3.2 Red: `processing_stages.feature` scenario "Refinement shows no raw transcript" replaces the dimmed one; `visual_feedback.feature` Pasted scenarios drop "with the pasted text"; steps in `app/tests/steps/`
- [x] 3.3 Green: `macos/pill_states.rs` and `macos/mod.rs` show the text window only in `Recording`; drop the preview; log the failure message at `warn` in `macos/failure_notice.rs`
- [x] 3.4 `just test`; commit

## 4. One pill size

- [x] 4.1 Red: unit tests in `macos/pill_content.rs` — `pill_width` is the same for every kind but the standby commands-unavailable pill, and every state's widest content fits it; run, see them fail
- [x] 4.2 Green: fixed pill width in `macos/pill_content.rs`, content centred
- [x] 4.3 `just test`; commit

## 5. Slower waveform

- [x] 5.1 Red: unit tests in `core/domain/level_history.rs` — one second adds five columns; 30 columns hold six seconds; run, see them fail
- [x] 5.2 Green: `COLUMN_PERIOD` 200 ms; fix the tests that assumed 100 ms
- [x] 5.3 `just test`; commit

## 6. Clipboard kept across the paste

- [x] 6.1 Red: `textpasting_macos_it` tests — an image, text and an empty clipboard are back after a paste; a copy made after the paste is not overwritten; run, see them fail; commit as intentionally red
- [x] 6.2 Green: snapshot and restore every pasteboard item and type with the change-count guard in `adapters/secondary/textpasting/macos/`
- [x] 6.3 `just test`; commit

## 7. Only the user's languages

- [x] 7.1 Red: `config/transcription.rs` unit tests — `languages` defaults to ru and en, an unknown code fails loading; `transcription_openai_it` tests with a stub service — a detected Korean answer is requested again with `ru` and its text used, a detected English answer is kept; run, see them fail; commit as intentionally red
- [x] 7.2 Red: `e2e-test/tests/features/speech_recognition.feature` scenarios "Short Russian speech is never transcribed as Korean" and "English speech stays English with automatic detection", with steps
- [x] 7.3 Green: `languages` setting, `verbose_json`, detected-language check and retry in `adapters/secondary/transcription/openai/`; add the setting to `README.md` and the setup CLI template
- [x] 7.4 `just test`; commit

## 8. Release

- [x] 8.1 Bump `version` in `app/Cargo.toml` to 0.24.1; `just check`; commit
- [ ] 8.2 Manual checks by the user: over a dark and a bright window the pill is clear glass with no top line, one size from record to Pasted; the live text shows below the pill while speaking and nowhere after; the waveform moves at half its old pace; a screenshot copied during a recording ended by voice is still in the clipboard; "окей окей" pastes in Russian
