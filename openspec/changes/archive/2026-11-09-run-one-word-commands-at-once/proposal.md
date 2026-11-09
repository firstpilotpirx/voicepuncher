## Why

A one-word command said after speech ("…commit it, enter") waits for the voice gate's pause before it counts, so "stop" of "stop the car" is no command; in a long utterance the gate hears that pause late, and the user hears the cue a second or more after the word. The service's log of 2026-11-09 (378 reports) shows a one-word "enter"/"stop" reported +0.47 s after its word when said alone, +0.95 s when it ends 2–10 s of speech (43 % over 1 s) and +1.4 s after 10–30 s (75 % over 1 s); until the report the dictation keeps recording. The user decided (2026-11-09) that a command word runs at once wherever it is said, before or after silence or speech, and keeps the command words out of dictation.

## What Changes

- **BREAKING** A command's one-word form (and its full phrase) is carried out as soon as a recognizer has decoded it, anywhere in an utterance: after silence, after speech, or followed by more speech. No pause after it is awaited. The words of a command inside a sentence ("we should stop the server") now trigger it: accepted by the user.
- A last word that may still grow into a longer command word ("start" of "restart") waits for the utterance to close, as today.
- When "stop" or "cancel" ends a dictation with speech after it, the words after it are not pasted, as with "stop stop" today.

Current stage: PoC. Target stage: PoC.
Scope: the decision of `command-spotting/src/streaming/decide/`, the placements of `command-spotting/src/matcher/`, their unit and session tests, the voice control specs and `.feature` scenarios that require a pause or forbid a command inside a sentence.
Acceptance bar: on the user's recorded sessions (`just streaming-bench --recorded`) every command still recognized exactly once and the median report delay of a one-word command after speech at most 0.4 s, from the log's 0.95–1.4 s; `just check` and `full-check` pass.
Out of scope: the English recognizer's speed (1 thread, ~1.1× real time: its own change), Whisper's time to finish the last chunk after "stop" (1.9 s since large-v3), the doubled and chained forms (already at once).
Interface: none.

## Capabilities

### Modified Capabilities

- `voice-control`: a command said at the end of an utterance or inside a sentence is carried out at once, without a pause after it.

## Impact

- `command-spotting/src/streaming/decide/`, `command-spotting/src/matcher/`, `command-spotting/src/streaming/session/tests/`, `e2e-test/tests/features/voice_control.feature`, `app/tests/features/` voice control scenarios.

## Release

SemVer: MINOR — a breaking change of when commands count.
Target version: 0.48.0.
