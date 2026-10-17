# Proposal

## Why

`just e2e-seams` (and so every `just full-check`) fails in 7 of the 22 `@seams` scenarios "Long speech under difficult recording conditions" of `e2e-test/tests/features/speech_recognition.feature`. They had never run: the recipe stopped earlier on the seam sweep, which `fix-dropped-phrase-at-word-pauses` (0.13.1) made pass. Measured on 2026-10-15 with `ggml-small.bin`:

- Russian, spoken slowly, with pauses between sentences, with high background noise, through a phone-quality line, clipped, with short audio dropouts: 10–19 % of the spoken words missing, against the spec's bar of at most one in ten. No phrase is dropped: the misses are misheard words ("очки задачи" for "очередь задач", "пополюдат" for "по полю дат").
- English with high background noise: "with the" printed twice — "how to split" was heard as "our test with the", so the check takes a misheard word for a phrase repeated at a seam.

The seam sweep (`chunked_transcription_it`) already shows Russian at 8.6–12 % missing words in normal conditions, so the bar sits at the edge of what the model does on synthesized Russian speech.

## What Changes

- Measure which lever brings every condition within the spec's bar, without making corpus WER worse: a larger or turbo whisper model, the recognition prompt (`restore-recognition-prompt`), decoding settings, or a bar that separates misheard words from words lost to chunking.
- Change what the measurements point to, so `just e2e-seams` and `just full-check` pass again.
- **Chosen after the measurements (2026-10-16, the user's pick):** dictations and recording files are recognized with the Whisper large-v3-turbo model (5-bit quantized); voice-command confirmations go to a second speech recognition service that keeps the small model. With large-v3-turbo all 22 difficult-conditions cases stay within the bar (worst: Russian with short dropouts, 8.6 % missing, against 19 % with small) and corpus WER on the 100 recordings drops from 0.116 to 0.077; decoding settings (beam 5, best of 5) left 6 of 22 cases over the bar. Confirming commands with large-v3-turbo as well made them ≈ 1–2 s slower and broke the command spotting tests, hence the second service.

## Capabilities

### Modified Capabilities

- `speech-recognition`: dictations and recording files are recognized with the large-v3-turbo model, so long speech under difficult recording conditions meets the accuracy bar of the spec; the confirmation of an utterance without speech goes to the speech recognition service for commands.
- `local-services`: the service scripts start a second speech recognition service, with the small model, that the command spotting service confirms commands with; the copies behind the app's address, the short-speech priority and a start of only the dictation service belong to the speech recognition service for dictation; slow, hung and quick confirmations name the speech recognition service for commands.
- `voice-control`: a confirmation that takes 15 seconds is one of the speech recognition service for commands; no promise changes.

## Impact

- `justfile`: the model download fetches large-v3-turbo; model selection for the two speech recognition services; start, stop and supervision of the second service; the command spotting service's speech recognition address.
- Memory: three copies of large-v3-turbo ≈ 2.2 GB (small: ≈ 2.0 GB), plus the copies of the speech recognition service for commands (≈ 0.7 GB each).
- Latency: a dictation request takes ≈ 0.5–0.8 s longer on an idle service (a 2.5–10 s clip: ≈ 1.0–1.2 s instead of 0.24–0.5 s; a 30 s chunk: 1.4 s instead of 0.9 s); command confirmations stay on the small model (how long they take while the dictation copies are busy on the same GPU is not measured yet).
- README (models, start command), `docs/ubiquitous-language.md` (the two speech recognition services).
- Tests: `just e2e-seams`, corpus accuracy (`just e2e-bench`), `just e2e-smoke` (command spotting with the second service), the local services scenarios.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:** `just e2e-seams` passes in full; corpus mean WER on the fixed sample of 100 no worse than today; `just full-check` passes.
- **Out of scope:** the hallucination filter and the seam merge.

## Release

MINOR: the spec's promise changes (the dictation model, a second speech recognition service). Target version 0.15.0 (from 0.14.2).
