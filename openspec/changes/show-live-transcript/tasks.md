# Tasks

## 1. The pipeline passes the live transcript to the overlay

- [ ] 1.1 Red: add `app/tests/features/component/live_transcript.feature` with the five component scenarios of design § Test levels and the pasted-text half of "Stopping a dictation clears its text", steps in `app/tests/steps/live_transcript.rs`; run `just test`, see them fail
- [ ] 1.2 Green: add the `LiveTranscriptView` port and `VisualIndicator::live_transcript_view` (default: a view that shows nothing), move a view into `run_live_worker`, show the merged text after each chunk with text; `just test` green
- [ ] 1.3 Add a unit test in `pipeline.rs` that a chunk without text shows nothing new; `just test` green
- [ ] 1.4 Add *Live transcript* and `LiveTranscriptView` to `docs/ubiquitous-language.md`; `just test` and commit

## 2. The overlay draws the live transcript

- [ ] 2.1 Red: unit tests for the tail cut (fits unchanged, cut at a word with `…`, one overlong word) in the macOS visual feedback adapter; see them fail
- [ ] 2.2 Green: implement the tail cut; `just test` green
- [ ] 2.3 Red: adapter integration tests in `app/tests/visualfeedback_macos_it.rs` — text shown while recording, hidden by a processing stage, by hide, cleared by the next recording, kept across a manual rebuild, dropped after the state left recording; see them fail
- [ ] 2.4 Green: add the text window to `WindowSet` and the live transcript state to `MacosVisualIndicator`; `just test` green and commit

## 3. Release

- [ ] 3.1 Bump `version` in `app/Cargo.toml` to 0.16.0; `just test`, `just check`, commit
- [ ] 3.2 Manual run by the user: a dictation over 30 s with the hotkey and one started by voice show their text under the pill after each part, the latest words for a long one, and no text after the stop, cancel or in the next dictation
