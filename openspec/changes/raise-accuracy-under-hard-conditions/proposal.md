# Proposal

## Why

`just e2e-seams` (and so every `just full-check`) fails in 7 of the 22 `@seams` scenarios "Long speech under difficult recording conditions" of `e2e-test/tests/features/speech_recognition.feature`. They had never run: the recipe stopped earlier on the seam sweep, which `fix-dropped-phrase-at-word-pauses` (0.13.1) made pass. Measured on 2026-10-15 with `ggml-small.bin`:

- Russian, spoken slowly, with pauses between sentences, with high background noise, through a phone-quality line, clipped, with short audio dropouts: 10–19 % of the spoken words missing, against the spec's bar of at most one in ten. No phrase is dropped: the misses are misheard words ("очки задачи" for "очередь задач", "пополюдат" for "по полю дат").
- English with high background noise: "with the" printed twice — "how to split" was heard as "our test with the", so the check takes a misheard word for a phrase repeated at a seam.

The seam sweep (`chunked_transcription_it`) already shows Russian at 8.6–12 % missing words in normal conditions, so the bar sits at the edge of what the model does on synthesized Russian speech.

## What Changes

- Measure which lever brings every condition within the spec's bar, without making corpus WER worse: a larger or turbo whisper model, the recognition prompt (`restore-recognition-prompt`), decoding settings, or a bar that separates misheard words from words lost to chunking.
- Change what the measurements point to, so `just e2e-seams` and `just full-check` pass again.

## Capabilities

### Modified Capabilities

- `speech-recognition`: long speech under difficult recording conditions meets the accuracy bar of the spec (or the bar is restated to measure what chunking loses, not what the model mishears).

## Impact

- `justfile` (model choice or whisper-server flags), possibly `e2e-test/src/speech.rs` (the repeated-phrase and missing-word checks) and the speech-recognition spec.
- Tests: `just e2e-seams`, corpus accuracy (`just e2e-bench`).

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:** `just e2e-seams` passes in full; corpus mean WER on the fixed sample of 100 no worse than today; `just full-check` passes.
- **Out of scope:** the hallucination filter and the seam merge.

## Release

PATCH when only flags, model choice or test checks change; MINOR if the spec's promise changes.
