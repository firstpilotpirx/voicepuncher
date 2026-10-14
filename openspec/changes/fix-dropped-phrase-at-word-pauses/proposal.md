# Proposal

## Why

A long dictation with pauses between words loses a whole phrase. The seam test `chunked_transcription_it::should_merge_chunked_speech_without_repeating_or_dropping_a_phrase` fails on `main` in 3 of its 30 conditions, all English speech with 350 ms pauses between words (lead-in 0, 0.7 and 1.4 s): a phrase of 26 words is missing from the pasted text ("…restarted the service. parts." — "and the response time dropped … into several small" is gone). The merge loses nothing (`lost_by_merge` 0); the speech recognition service drops the phrase inside one chunk (missing-word share 19.6 %). Every `just full-check` fails on it, so a real regression at the seams would go unseen.

The archived changes `cut-chunks-at-pauses` and `keep-words-after-pauses` are in the code (chunks are cut in pauses, whisper-server runs with timestamps). Since then whisper-server runs with its own voice activity detection (`--vad -vsd 2000 -vp 500`), added so a silent recording yields no text — and `keep-words-after-pauses` measured whisper-server's voice activity detection at up to 36 % missing words on the same seam matrix. That is the first suspect.

## What Changes

- Find why the phrase is dropped: measure the seam matrix and the benchmark corpus with whisper-server's voice activity detection off, and with other silence and padding settings.
- Change what the measurements point to (the service flags in the `justfile`, or the app's chunking), so no chunk drops a phrase, while a silent recording still yields no text.
- No change to voice control, the hotkey or the pasted text's form.

## Capabilities

### Modified Capabilities

- `speech-recognition`: long speech with pauses between words keeps every phrase.

## Impact

- `justfile` (`start-model-servers`: whisper-server flags) and possibly `app/src/core/application/chunking.rs`.
- Tests: the existing seam sweep and seam matrix (`just e2e-seams`), corpus accuracy, the silent-recording check.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:** `just e2e-seams` passes (every condition of the seam matrix, no repeated or dropped phrase); corpus mean WER on the fixed sample of 100 no worse than today; a silent recording still yields no text; `just full-check` passes in full.
- **Out of scope:** a different speech recognition model; voice control; changes to the seam merge.

## Release

PATCH — a bug fix; target version **0.10.1** (after `commands-without-pauses` releases 0.10.0; if `main` moved on, the next patch).
