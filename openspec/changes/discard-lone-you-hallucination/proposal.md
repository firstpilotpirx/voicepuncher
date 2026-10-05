# Proposal

## Why

An accidental recording of silence pastes the word "you" into the focused app. With automatic language detection, the speech recognition service (whisper-server) returns "you" for 1 s and 10 s of digital silence and "you you" for 60 s. The old in-process engine did the same, so this is not a regression, but the hallucination filter only knows subtitle credits, subscription calls, blank-audio markers and repetition loops of 10 or more words, so a lone "you" slips through.

## What Changes

- A transcript made only of the word "you" — any number of times, case-insensitive, punctuation and whitespace ignored — counts as a known recognition hallucination and is discarded instead of pasted.
- Transcripts that contain "you" alongside other words ("you know", "thank you", "Hello, how are you doing?") are unchanged.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `speech-recognition`: the requirement "Known recognition hallucinations are discarded" adds a transcript consisting only of "you" to the list of known hallucinations, with a scenario that reproduces the bug from a silent recording.

## Impact

- Code: `app/src/core/application/transcript_filter.rs` (`is_whisper_hallucination`) plus its unit tests; the OpenAI-API adapter already calls it, so no adapter change.
- Tests: unit tests for the filter; an E2E scenario in `e2e-test/tests/features/speech_recognition.feature` that transcribes silence through the running service and expects no text.
- Docs: none beyond the spec.
- Trade-off: a user who deliberately dictates only the word "you" gets nothing pasted. Acceptable — a one-word "you" dictation is far rarer than an accidental silent recording.

## Scope

- **Lifecycle stage:** PoC, stays PoC. Acceptance bar: unit tests and the E2E silence scenario pass in `just check`; `just full-check` passes before merge.
- **Out of scope:** other single-word English hallucinations ("Thank you.", "Bye.", "Okay."); a voice-activity gate that skips silent recordings before the service is called; changing the service's language or decoding settings.

## Release

PATCH — bug fix. Based on PR #11 (`feat/externalize-speech-recognition`, version 0.2.0); target version **0.2.1**.
