## 1. Test speech and seam metrics

- [x] 1.1 Red: unit tests in `e2e-test/src/speech.rs` for `repeated_phrases` (finds a pair repeated at a seam, ignores pairs the speech itself repeats, ignores case and punctuation) and `missing_word_share` (0 for identical text, counts dropped words); run, see them fail
- [x] 1.2 Green: implement `repeated_phrases`, `missing_word_share` and word normalization
- [x] 1.3 Add the Russian and English paragraphs, the 11 recording conditions and `synthesize(lang, condition, lead_in_secs)` (macOS `say` at 16 kHz, deterministic noise, band-pass, gain, clipping, dropouts); unit test that every condition yields 16 kHz audio longer than 30 s
- [x] 1.4 `just check`, commit

## 2. Chunk boundary rule in the core

- [x] 2.1 Red: unit tests in `chunking.rs` for the cut-point function (hard boundary without silence, latest quiet frame within the lookback) and `split_into_chunks` (short recording → one chunk; 70 s tone → three chunks, each after the first starting the overlap before the previous cut; cut at a silence inside the lookback); run, see them fail
- [x] 2.2 Green: move the silence search from `recording/cpal` into `chunking.rs`, add `split_into_chunks`; the cpal live watcher calls the core function; move the two cpal VAD tests
- [x] 2.3 `just check`, commit

## 3. Recording files are transcribed in chunks

- [x] 3.1 Red: scenario "Recording file longer than the chunk threshold" (RU + EN examples, `@smoke`) in `e2e-test/tests/features/speech_recognition.feature` with steps (synthesized `normal` speech, file mode without refinement, stderr names the chunks, no repeated phrase, missing share ≤ 0.10); pipeline unit test that `run_once` transcribes a 70 s recording as three chunks and pastes their merge; run, see them fail
- [x] 3.2 Green: `Pipeline::run_once` splits with `split_into_chunks` above the threshold, transcribes in order, skips dropped chunks, merges, logs `file mode: N chunks`
- [x] 3.3 `just check`, commit

## 4. Reproduce duplicated text at chunk seams

- [x] 4.1 Red: unit seam fixtures in `chunking.rs` (test-only table): real whisper-server seams in Russian and English across conditions and seam positions, plus the corpus seams from the proposal; each asserts the reviewed merge
- [x] 4.2 Red: unit cases for the rules — punctuation and case, `ё`/`е`, hyphenated vs joined spelling, word cut at the edge, different reading of one word, disjoint chunks keep every word, a phrase repeated outside the overlap survives, inner empty chunk
- [x] 4.3 Red: `app/tests/features/component/speech_recognition.feature` with scenarios "Dictation longer than the chunk threshold", "Chunks transcribe the overlap differently" (outline: RU + EN × punctuation/case, spelling, cut-off word, different reading) and "Chunks share no words"; steps in `app/tests/steps/speech_recognition.rs` drive the `Pipeline` with a stub recorder that delivers live chunks, a stub transcriber returning the chunk texts, and a capturing paster
- [x] 4.4 Red: adapter integration test `app/tests/chunked_transcription_it.rs` (gated on `VOICEPUNCHER_TRANSCRIPTION_URL`): synthesized RU + EN speech × `normal`, `slow`, `fast`, `sentence_pauses`, `word_pauses` × lead-in 0, 0.7, 1.4 s; split with `split_into_chunks`, transcribe with the real service through `OpenAiTranscriber`, merge; assert no repeated phrase and that the merge drops ≤ 2 % of the words the chunks recognized; add `voicepuncher-e2e-test` as a dev-dependency of `app`; run it in `e2e-smoke`
- [x] 4.5 Red: scenario outline "Long speech under difficult recording conditions" in `e2e-test/tests/features/speech_recognition.feature` (RU + EN × 11 conditions, `@seams`) with steps; recipe `just e2e-seams`
- [x] 4.6 Run all of the above, see the seam tests fail on duplicated phrases; commit the failing tests on their own (`test: reproduce duplicated text at chunk seams`, message states it is intentionally red)

## 5. Tolerant seam merge

- [x] 5.1 Green: implement the D1 alignment in `merge_chunk_texts`; `just test` and the integration and E2E seam tests pass
- [x] 5.2 Refactor: drop the unused overlap-fraction argument and its computation in the pipeline; keep every test green
- [x] 5.3 `just check`, commit

## 6. Checks and docs

- [x] 6.1 `full-check` runs `e2e-seams`; `AGENTS.md` and `README.md` list the recipe
- [x] 6.2 `docs/ubiquitous-language.md`: add **Chunk seam** (where two overlapping chunks meet; the merge keeps the overlap once) and update **Chunk threshold** (dictations and recording files)
- [x] 6.3 `just check`, commit

## 7. Release

- [x] 7.1 Bump `version` in `app/Cargo.toml` to `0.3.1`, `just check`, commit
- [ ] 7.2 Manual check by the user: one dictation longer than 30 s (chunks are logged, the pasted text repeats no phrase at the seam)
