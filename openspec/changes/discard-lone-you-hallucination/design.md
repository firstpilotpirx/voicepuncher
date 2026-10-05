# Design

## Context

`is_whisper_hallucination` in `app/src/core/application/transcript_filter.rs` checks a lowercased transcript against substring patterns and a repetition-loop rule that needs at least 10 words. The OpenAI-API adapter (`app/src/adapters/secondary/transcription/openai/mod.rs`) calls it on every service answer and returns `None` on a hit, so nothing is refined or pasted. "you" cannot become a substring pattern (it would drop "how are you"), and "you you" is too short for the loop rule.

## Goals / Non-Goals

**Goals:**
- Discard transcripts whose only words are "you".

**Non-Goals:**
- A general list of single-word hallucinations, or a silence detector before the service call.

## Decisions

- **Whole-transcript word check, separate from the substring patterns.** Split the lowercased transcript on whitespace, strip every non-alphanumeric character from each token, drop tokens that become empty, and report a hallucination when at least one token is left and every token equals `you`. Alternatives: a regex (`^\W*(you\W*)+$`) — needs the `regex` crate, which the core does not use; adding `"you"` to the patterns — matches real speech.
- **Tokens of only punctuation are ignored**, so `"You. - You."` and `"you…"` count; a transcript of only punctuation stays "not a hallucination" (same as today for empty text) and is handled by the existing empty-transcript path.
- **Kept in the core filter**, not the adapter, so every speech recognition adapter gets it.

## Test levels

| Spec scenario | Level |
|---|---|
| Silence produces a subtitle credit | unit (`transcript_filter.rs`) and adapter integration (`transcription_openai_it.rs`), unchanged |
| Silence produces a lone "you" | unit (filter on "you", "You.", "you you", "YOU, you!") and E2E `.feature` (silent WAV of 1/10/60 s through the running service with automatic language, no refinement, stdout empty) |
| Speech containing "you" is kept | unit ("you know", "thank you", "Hello, how are you doing?" not filtered) |

## Risks / Trade-offs

- [A deliberate one-word "you" dictation is dropped] → accepted; noted in the proposal.
- [E2E depends on the model actually returning "you" for silence] → the scenario asserts the user-visible outcome (no text), which holds whether the service returns "you", an empty text or a blank-audio marker; the unit tests pin the "you" case itself.
- [E2E runtime: 60 s of silence per run] → transcribed without refinement, a few seconds on Metal; tagged `@smoke` so it runs in `just check` like the existing silence outline.
