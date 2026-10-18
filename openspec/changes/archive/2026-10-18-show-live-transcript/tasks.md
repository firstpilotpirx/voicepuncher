# Tasks

## 1. The pipeline passes the live transcript to the overlay

- [x] 1.1 Red: add `app/tests/features/component/live_transcript.feature` with the five component scenarios of design § Test levels and the pasted-text half of "Stopping a dictation clears its text", steps in `app/tests/steps/live_transcript.rs`; run `just test`, see them fail
- [x] 1.2 Green: add the `LiveTranscriptView` port and `VisualIndicator::live_transcript_view` (default: a view that shows nothing), move a view into `run_live_worker`, show the merged text after each chunk with text; `just test` green
- [x] 1.3 Add a unit test in `pipeline.rs` that a chunk without text shows nothing new; `just test` green
- [x] 1.4 Add *Live transcript* and `LiveTranscriptView` to `docs/ubiquitous-language.md`; `just test` and commit

## 2. The overlay draws the live transcript

- [x] 2.1 Red: unit tests for the tail cut (fits unchanged, cut at a word with `…`, one overlong word) in the macOS visual feedback adapter; see them fail
- [x] 2.2 Green: implement the tail cut; `just test` green
- [x] 2.3 Red: unit tests in the macOS visual feedback adapter for what the text window shows — the text while recording, nothing in a processing stage, the wait for focus or hidden, nothing after a new recording starts, nothing for an update after the state left recording; see them fail
- [x] 2.4 Green: add the text window to `WindowSet` and the live transcript state to `MacosVisualIndicator`; `just test` green and commit

## 3. Release

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.17.0; `just test`, `just check`, commit
- [x] 3.2 Manual run by the user: a dictation over 30 s with the hotkey and one started by voice show their text under the pill after each part, the latest words for a long one, and no text after the stop, cancel or in the next dictation
