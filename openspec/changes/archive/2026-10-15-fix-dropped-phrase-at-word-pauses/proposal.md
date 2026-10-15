# Proposal

## Why

A long dictation with pauses between words loses a whole phrase. The seam test `chunked_transcription_it::should_merge_chunked_speech_without_repeating_or_dropping_a_phrase` fails on `main` in 3 of its 30 conditions, all English speech with 350 ms pauses between words (lead-in 0, 0.7 and 1.4 s): a phrase of 26 words is missing from the pasted text ("…restarted the service. parts." — "and the response time dropped … into several small" is gone). Every `just full-check` fails on it, so a real regression at the seams would go unseen.

The proposal first suspected whisper-server's voice activity detection. Measurements rule it out: with voice activity detection off the same 26 words are missing in the same 3 conditions, and greedy or beam decoding, temperature fallback and non-speech token suppression change nothing. The speech recognition service transcribes the chunk in full, but appends the classic hallucination "Thanks for watching." in the silence after the last word of the chunk. The app's hallucination filter discards a transcript that *contains* a known hallucination phrase, so the whole chunk — 26 real words — is thrown away. The spec already says only a transcript that *consists only of* a hallucination is discarded; the code does not do that.

## What Changes

- A chunk transcript that holds real speech plus a known hallucination phrase keeps the speech: the sentence holding the hallucination phrase and blank-audio or music markers are removed, and the rest is passed on.
- A transcript that is nothing but hallucinations (credits, subscription calls, markers, a lone "you", a repetition loop) is still discarded, so a silent recording still yields no text.
- whisper-server flags in the `justfile` stay as they are (measured: they are not the cause).
- No change to voice control, the hotkey, the seam merge or the pasted text's form.

## Capabilities

### Modified Capabilities

- `speech-recognition`: a known hallucination appended to real speech is removed instead of discarding the speech; long speech with pauses between words keeps every phrase.

## Impact

- `app/src/core/application/transcript_filter.rs` (the hallucination filter) and its caller in the speech recognition adapter (`app/src/adapters/secondary/transcription/openai/`).
- Tests: unit tests of the filter, the existing seam sweep and seam matrix (`just e2e-seams`), a new E2E scenario for English word-paused speech after different lead-ins, the silent-recording scenarios, corpus accuracy.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:** the seam sweep passes in every condition (no repeated or dropped phrase), and so does the new E2E scenario; corpus mean WER on the fixed sample of 100 no worse than today; a silent recording still yields no text; `just full-check` passes except for the "Long speech under difficult recording conditions" scenarios, which never ran before this fix and fail on misheard words — they go to `raise-accuracy-under-hard-conditions` (decided by the user on 2026-10-15).
- **Out of scope:** a different speech recognition model; whisper-server flags; voice control; changes to the seam merge; new hallucination phrases.

## Release

PATCH — a bug fix; target version **0.13.1** (the next patch after `main`'s 0.13.0; if `main` moved on by merge time, the next patch on top of it).
