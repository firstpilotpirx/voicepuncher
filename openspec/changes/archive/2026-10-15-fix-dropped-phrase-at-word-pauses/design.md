## Context

See proposal.md § Why for the measurements. The speech recognition adapter (`OpenAiTranscriber::transcribe`) passes every service answer through `transcript_filter::is_whisper_hallucination`, which returns true when the lowercased transcript *contains* any known hallucination phrase or marker, or is a repetition loop, or is only "you". On true the whole transcript is dropped (`Ok(None)`). A 30 s chunk of word-paused English speech ends in silence; whisper transcribes all of it and adds "Thanks for watching." as its own last segment, so 26 real words are dropped.

The adapter receives the transcript as one line: `normalize_transcript` removes the `\n` between whisper-server's segments, because segments may split a word.

## Goals / Non-Goals

**Goals:**
- Keep every spoken word of a transcript that also holds a hallucination phrase or marker.
- Keep discarding transcripts that are hallucination only, so silence still yields no text.

**Non-Goals:**
- New hallucination phrases, or a different detection for loops and "you".
- whisper-server flags (measured: voice activity detection, beam search, temperature fallback and non-speech token suppression do not change the result).
- Segment timestamps or `verbose_json` from the service.

## Decisions

### Remove the hallucination, not the transcript

`transcript_filter` gets `strip_whisper_hallucinations(text) -> Option<String>`: it returns the transcript without its hallucinations, or `None` when nothing that is speech is left. The adapter calls it instead of `is_whisper_hallucination` and logs what it removed or dropped.

1. **Markers** (`[music]`, `[applause]`, `[blank_audio]`, `(music)`, `(applause)`) are removed as tokens, wherever they are: they never stand for a spoken word.
2. **Phrases** (subtitle credits, subscription calls, "продолжение следует", …) remove the **sentence** that holds them. A sentence ends after `.`, `!`, `?` or `…`; text after the last such mark is a sentence too. The sentence, not just the phrase, goes because a credit carries a name ("Субтитры создавал DimaTorzok") that is part of the hallucination.
3. The remainder is joined with single spaces. It is `None` when it is empty, when it is a repetition loop, or when it is only "you" — the existing whole-transcript checks, now on the remainder.

Alternatives considered:
- *Remove only the phrase itself.* Leaves the credit's name ("DimaTorzok") behind as text.
- *Filter per whisper segment* (keep the `\n` separators or ask for `verbose_json`). The hallucination is usually a segment of its own, but segments may split a word, and the adapter's one-line transcript contract would change; the sentence rule needs neither.
- *Stop whisper from hallucinating* (flags, voice activity detection, trimming trailing silence from the chunk). Measured without effect for the flags; trimming silence changes the chunk audio the seam merge relies on and does not remove hallucinations in mid-recording pauses.

### `is_whisper_hallucination` goes

Its only caller is the adapter; keeping both functions would keep the "contains" rule alive. Its unit tests move to `strip_whisper_hallucinations` with the same inputs (hallucination only → `None`, speech → unchanged).

## Test levels

| Scenario | Level |
|---|---|
| Silence produces a subtitle credit | adapter integration (`app/tests/transcription_openai_it.rs`, stub service; existing `should_discard_known_hallucination`) and unit (`transcript_filter.rs`) |
| Silence produces a lone "you" | E2E (`e2e-test/tests/features/speech_recognition.feature`, existing `@smoke` outline) and unit |
| Speech containing "you" is kept | unit (`transcript_filter.rs`, existing cases moved to the new function) |
| Hallucination after speech is removed and the speech is kept | unit and adapter integration (stub answers speech plus "Thanks for watching."): the filter runs inside the adapter, and the component tests stand in for the adapter |
| Marker inside speech is removed and the speech is kept | unit and adapter integration (stub answers speech with "[BLANK_AUDIO]" inside) |
| Speech with pauses between words or sentences | E2E (`speech_recognition.feature`, existing `@seams` outline) |
| Speech after pauses wherever the seams fall | adapter integration against the real service (`app/tests/chunked_transcription_it.rs`, existing seam sweep, run by `just e2e-seams`) |
| English speech with a short pause after every word keeps every phrase | E2E (`speech_recognition.feature`, new `@seams` outline with lead-ins of 0, 0.7 and 1.4 s); it reproduces the bug on `main` |

## Risks / Trade-offs

- [A spoken sentence that contains a listed phrase ("thanks for watching" said on purpose) is removed] → Before, the whole chunk was removed; now only that sentence. Acceptable for a dictation tool.
- [whisper glues a hallucination to speech without a sentence mark ("…small parts thanks for watching")] → The whole run-on sentence goes, as before for the whole chunk; never worse than today.
- [Corpus WER moves] → The bench (100 recordings) runs in `full-check`; mean WER must not get worse.

## Migration Plan

None: no configuration or service change. Rollback is reverting the commit.
