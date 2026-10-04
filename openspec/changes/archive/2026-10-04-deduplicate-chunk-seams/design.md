# Design

## Context

- `chunking::merge_chunk_texts` joins chunk transcripts by dropping the longest run of words (at most 20) that is byte-identical at the end of the text so far and the start of the next chunk. Its `_overlap_word_fraction` argument is unused.
- The chunk boundary rule lives in the microphone adapter (`recording/cpal`, `live_watcher_thread` + `vad_cut_point_native`): at the hard boundary (`chunk_threshold_secs` after the previous cut) it searches back up to `vad_lookback_secs` for the latest 20 ms frame quieter than `vad_rms_threshold` and cuts there (or at the hard boundary); the next chunk starts `chunk_overlap_secs` before the cut.
- File mode (`Pipeline::run_once`) sends the whole recording in one request, so no E2E test reaches the seam merge.
- whisper-server runs with `-nt` (no timestamps), so the merge can only use text.
- Defaults: threshold 30 s, overlap 2 s, lookback 5 s, RMS threshold 0.01.

## Goals / Non-Goals

**Goals:**
- The overlap of two chunks appears once in the transcript even when the two transcripts of it differ.
- No word is dropped where chunks share no confident overlap.
- One chunk boundary rule for dictations and recording files, so recording files exercise the seams end to end.
- Seam regressions are caught at unit, component, adapter integration and E2E level, in Russian and English, under many recording conditions and seam positions.

**Non-Goals:**
- Tuning threshold, overlap or the silence detector.
- Word timestamps from the service.
- Removing duplicates the service produces inside one chunk.

## Decisions

### D1. Seam merge = local alignment of normalized words

For each next chunk: take the last 24 words of the text so far (`tail`) and the first 24 words of the chunk (`head`). Normalize each word: lowercase, `ё`→`е`, drop every non-alphanumeric character (punctuation, hyphens, quotes). Two words match when:

- their normalized forms are equal, or
- one is a prefix of the other and the shorter has at least 3 characters (a word cut at the chunk edge: `чека`/`чекап`), or
- their character similarity (1 − Levenshtein / longer length) is at least 0.6 (`раундробинам`/`раундробином`).

Run a Smith–Waterman local alignment between `tail` and `head` (match +2 × similarity, mismatch −1.5, gap −1). Of all alignments whose end in `tail` lies within the last 4 words, take the best-scoring one. Accept it only if:

- it starts within the first 4 words of `head` (the overlap lies at the start of the next chunk), and
- it holds at least 2 matched words, or exactly 1 matched word of 5 or more characters that sits at the very end of `tail` and at the start of `head` (within 1 word).

When accepted, the merged text is `text[..end of match in tail] + head[end of match in head..] + rest of chunk`. The words of `tail` after the match and the words of `head` before it lie in the overlap audio, which the other chunk covers; they are the partial or misheard edge words. Otherwise the chunk is appended whole, as today.

Alternatives considered:
- *Normalize and keep the exact suffix/prefix match*: fixes punctuation and case only; cut-off and misread words (4 of the 10 corpus seams) still duplicate.
- *Drop the overlap only when the cut fell in silence*: the cut "silence" is a single 20 ms frame and often lies inside a word, so a word would be lost; with noise there is no silence and the overlap is still needed.
- *Word timestamps*: needs whisper-server with timestamps on, which changes the decoding settings the accuracy bar was measured with.
- *Ask the LLM to remove the repetition*: refinement is optional (`--no-llm`) and must not change content.

The 24-word windows and the 4-word slack cover 2 s of overlap at fast speech (≈ 5–7 words) plus edge noise. Measured on 37 recordings (10 corpus + 26 synthesized + 1 offset), the prototype removed every duplicate (81 duplicated word pairs before) and lost no more words than the exact merge.

### D2. Chunk boundary rule moves into the core

`chunking.rs` gets a pure function that, given the samples so far, the sample rate, the previous cut and `ChunkingCfg`, returns the next cut point (the silence search moves verbatim from the cpal adapter), plus `split_into_chunks(&AudioBuffer16k, ChunkingCfg) -> Vec<AudioBuffer16k>` that applies it to a whole recording, adding the overlap before every chunk after the first. The cpal live watcher calls the core function on its native-rate buffer instead of its own copy. No framework code, so the architecture test keeps passing.

### D3. File mode transcribes in chunks

`Pipeline::run_once` splits the recording with `split_into_chunks` when the chunk threshold is above zero and the recording is longer than it; it transcribes the chunks in order, skips chunks the hallucination filter dropped, merges with D1 and continues with refinement and paste as before. It logs `file mode: N chunks` so a test can see the recording was chunked. A threshold of 0 keeps single-pass transcription.

### D4. Shared test speech and seam metrics

`voicepuncher-e2e-test` (library) gets a `speech` module used by the E2E steps and, as a dev-dependency of `app`, by the adapter integration test:

- two fixed paragraphs, Russian (voice Milena) and English (voice Eddy), ≈ 45 s at normal rate;
- `synthesize(lang, condition, lead_in_secs) -> Vec<i16>` with `say --file-format=WAVE --data-format=LEI16@16000`; conditions: `normal` (190 wpm), `slow` (140), `fast` (260), `sentence_pauses` (1.5 s), `word_pauses` (350 ms), `noise_low`, `noise_high` (deterministic noise, amplitude 0.01 / 0.05), `phone_band` (300–3400 Hz band-pass, 8 kHz resample), `quiet` (×0.15), `clipped` (×6, clamped), `dropouts` (120 ms of zeros every 0.6–1.7 s, fixed seed). `lead_in_secs` prepends silence to move the seams to other words;
- `repeated_phrases(spoken, transcript) -> Vec<String>`: normalized word pairs that occur more often in the transcript than in the spoken text;
- `missing_word_share(spoken, transcript) -> f64`: share of spoken words not aligned to the transcript (LCS over normalized words).

Pass criteria: no repeated phrase, and missing share ≤ 0.10 for recording files (E2E). The adapter integration test sees the chunk transcripts, so it measures the merge alone: the words it drops that some chunk recognized (missing share of the merge minus missing share of all chunk transcripts joined) stay ≤ 0.02. Whisper itself drops words inside a chunk after long pauses (up to 15 % with 350 ms between words); that is recognition, not the merge, and out of scope.

### D5. Real seam fixtures for unit tests

The unit tests replay seams captured from whisper-server output during the investigation: the end of chunk *n* and the start of chunk *n+1*, with the merge expected after review, across both languages, all conditions and several seam positions (dozens of cases), plus the corpus seams quoted in the proposal. They live next to `chunking.rs` as a test-only table, so they run in `just test` in milliseconds.

### Test level per spec scenario

| Scenario | Level |
|---|---|
| Dictation longer than the chunk threshold | component `.feature` (`app/tests/features/component/speech_recognition.feature`): pipeline with a stub transcriber returning overlapping chunk texts; unit tests for `split_into_chunks` |
| Chunks transcribe the overlap differently | unit (real seam fixtures, RU + EN) + component `.feature` outline (RU + EN × punctuation/case, spelling, cut-off word, different reading) |
| Chunks share no words | unit + component `.feature` |
| Recording file longer than the chunk threshold | E2E `.feature` (`e2e-test/tests/features/speech_recognition.feature`, `@smoke`, RU + EN); unit test for `run_once` chunking |
| Long speech under difficult recording conditions | E2E `.feature` outline RU + EN × 11 conditions (`@seams`, run by `just e2e-seams` in `full-check`) |
| Chunk seams fall on different words | adapter integration `app/tests/chunked_transcription_it.rs`: real service, RU + EN × 5 `say` conditions × 3 lead-ins (gated on the service URL, run in `e2e-smoke`) |

Hotkey, recording, overlay, paste and permissions code is untouched except the cpal watcher calling the moved cut function, so the manual run covers one long dictation (> 30 s) only.

## Risks / Trade-offs

- **A legitimately repeated phrase across a seam is collapsed** ("да, да" spoken across the seam). Mitigation: the alignment must sit in the overlap region (end of `tail`, start of `head`); outside that, repeats survive. Accepted for a 2 s window.
- **A false match drops up to 4 edge words.** Mitigation: the ≥ 2-word or long-single-word rule; E2E missing-word bar of 10 %; unit cases with disjoint chunks.
- **Chunking recording files changes E2E accuracy numbers** for the 10 corpus recordings longer than 30 s. Measured effect is fewer lost words; `full-check` confirms the mean WER does not regress.
- **`say` voices differ between macOS versions**, so synthesized audio may change after an OS update. The E2E criteria are relative to the spoken text, not to a fixed transcript, so they stay valid; the unit fixtures are text-only and unaffected.
- **Runtime of the checks:** the integration sweep adds ≈ 1–2 min to `e2e-smoke`; the 22-example E2E matrix (≈ 3 min) runs in `full-check` only.
