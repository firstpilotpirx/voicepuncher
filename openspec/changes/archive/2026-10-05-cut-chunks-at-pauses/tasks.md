# Tasks

## 1. Cut in the longest pause, quiet level from the noise floor

- [x] 1.1 Add `synthesize_with_clean` to `e2e-test/src/speech.rs` (the speech after each lead-in, with the same speech before `degrade`; `say` runs once), with a unit test that the clean copy has the degraded copy's length; verify `just test`
- [x] 1.2 Red: `app/tests/chunk_cuts_it.rs` for the scenario "Most seams of synthesized speech fall in pauses": Russian and English, every condition but the phone line, lead-ins 0–2.8 s in steps of 0.35 s, default chunking config; a cut is in a pause when the clean frame there is below 0.003 RMS inside a run of ≥ 100 ms; cuts at the recording end are skipped; assert ≥ 80 % and print the share per condition; run it and see it fail on the current rule (spike: 28 %)
- [x] 1.3 Red: in `chunking.rs` add unit tests for the scenarios "Chunk ends in a pause rather than in a quiet moment of a word", "Chunk ends in the longest pause", "Pause is found under background noise", "Quiet speech is not taken for a pause" and "Short audio dropout is not a pause" on synthetic samples; run `just test` and see each fail on the current rule for the expected reason
- [x] 1.4 Green: rewrite `next_cut` per design D1–D2 (frame RMS from `last_cut` to the hard boundary, quiet level `max(0.002, min(2 × 10th percentile, 0.5 × median))`, runs ≥ 160 ms inside the lookback, middle of the longest, latest on ties, hard boundary otherwise) with named constants; drop `vad_rms_threshold` from `ChunkingCfg` and its callers; adjust the existing cut tests to the pause rule (the "no pause → hard boundary" test keeps its meaning); verify `just test` and `chunk_cuts_it` are green
- [x] 1.5 Refactor with `just test` green; add the terms **Pause** and **Noise floor** to `docs/ubiquitous-language.md` (code: `next_cut`), verified by reading the entries against the code
- [x] 1.6 `just check` passes; commit `fix(chunking): cut chunks in the longest pause, judged against the noise floor`

## 2. Remove the fixed silence level from the configuration

- [x] 2.1 Red: unit test in `config/transcription.rs` that a `[transcription]` table setting `vad_rms_threshold` is refused with a message saying pauses follow the recording's background noise level and the setting can be deleted, and that the `[whisper]` hint for `vad_rms_threshold` says to delete it; add the `@smoke` scenario "Config still sets a fixed silence level" to `e2e-test/tests/features/speech_recognition.feature` under "Outdated speech recognition settings are reported" with its steps in `e2e-test/tests/steps/speech_recognition.rs`; run `just test` and see the unit test fail
- [x] 2.2 Green: remove `TranscriptionCfg::vad_rms_threshold`, add `reject_removed_transcription_keys` called from the config loader next to `reject_removed_whisper_section`, change the `[whisper]` hint, remove the key from `app/config.example.toml`; verify `just test` is green and the new scenario passes in `just e2e-smoke`
- [x] 2.3 `just check` passes; commit `fix(config): refuse the removed vad_rms_threshold setting`

## 3. Bound every chunk by the threshold

- [x] 3.1 Red: unit test in `chunking.rs` for the scenario "No chunk is longer than the chunk threshold": `split_into_chunks` of speech more than twice the threshold long yields no chunk longer than the threshold; run it and see it fail (a 32 s middle chunk)
- [x] 3.2 Green: `next_cut` takes the chunk start and puts the hard boundary at chunk start + threshold (design D5); `split_into_chunks`, the live watcher in `recording/cpal` and `chunk_cuts_it.rs` cut while more than a threshold of audio follows the chunk start; adjust `should_split_a_long_recording_into_overlapping_chunks` to 30/30/14 s; verify `just test`
- [x] 3.3 Remove the `@wip` seam example again and make the E2E recipes skip `@wip` under `--tags` (`'@x and not @wip'` in the `justfile`); verify with the E2E runs of `just full-check`
- [x] 3.4 `just full-check` (a superset of `just check`) passes; commit `fix(chunking): keep every chunk, overlap included, within the chunk threshold`

## 4. Release

- [x] 4.1 Bump `version` in `app/Cargo.toml` to `0.5.2` (planned as `0.4.2`; `main` reached `0.5.1` meanwhile); verify `kit version-check cut-chunks-at-pauses` and `just test`
- [x] 4.2 Run `just e2e-seams` (22/22) and the seam sweep of `chunked_transcription_it.rs`; compare repeated phrases and lost words with 0.4.1, and corpus mean WER from `just check` with 0.4.1; record the numbers in the PR body
- [x] 4.3 `just check` passes; commit `chore(release): 0.4.2`, then `0.5.2` after merging `main`
- [x] 4.4 Manual check the user performs (the live watcher of the recording adapter changed): `cargo run --release -p voicepuncher`, dictate for longer than the chunk threshold (about a minute, with pauses) and paste the log; a healthy run logs every chunk transcribed and the full text pasted
