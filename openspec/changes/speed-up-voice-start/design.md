# Design

## Context

See proposal.md § Known and suspected causes. The keyword spotter reports "start" ≈0.25 s after its words, but a running utterance is confirmed only by timed checks. The lead-in already comes from the last 10 s of streamed audio, so only the cue moves.

## Decisions

**D1 — Measure first, with the timing log.** The timing log (D5) and an E2E latency harness land first; today's numbers per placement go into this file before any fix. The conditional part of D6 lands only if the numbers still miss the bar. Rejected: fixing from estimates.

**D2 — A spotted start triggers a check.** When the spotter reports "start dictation" or "restart dictation" in a running utterance, the service asks a check once 0.35 s of audio follows the phrase, so the next word has begun ("start stop" must not start). It uses the head clip, or the last 2.5 s of a longer utterance, and the head and window matchers; "start" heard alone is taken only from a second check 0.3 s later. While another check waits, it is asked when that one answers: at most one check waits (`local-services`). Timed checks stay as a fallback. Rejected: starting on the spotter's report alone (false starts). Load: one ≤2.5 s confirmation per spotted start.

**D3 — The head check takes doubled starts and chains.** It also applies the "anywhere" matcher, so "start start" and "enter start" count at once.

**D4 — Segment confirmations do not wait behind checks.** Each connection gets a second confirmation thread for checks; segments, unheld phrases, tails and re-hearings keep the first. At most two requests per connection are in flight, for three speech recognition copies. Rejected: cancelling a running check (an HTTP request cannot be withdrawn).

**D5 — The timing log.** The app's heard audio records when each streamed block was captured, so a stream position maps to its capture time even after audio was lost. Command and phrase events carry an optional `timing`: the stream position at which their confirmation was asked and its duration. At the cue the app logs `voice start: total=… hear=… confirm=… deliver=… cue=…` in ms, from the capture time of the command's end.

**D6 — "restart" does not wait for the dropped dictation.** Cancel stops the recorder without flushing and plays the cues at once; the new live session takes the transcriber back from the old worker before its first chunk. Only if D1's numbers miss the bar: confirm a lone start during its pause, and show the overlay before the recorder opens.

## Risks / Trade-offs

- [A cut next word gives a false start] → the 0.35 s lead, the second check, every false-start scenario.
- [Extra load] → one check per spotted start; `just full-check` runs accuracy and seams with it.
- [Two threads reorder answers] → the outbox orders events by job.

## Dependencies

None declared yet; stage `dependencies` re-judges them.

## Test levels

| Scenario | Level |
|---|---|
| Start start followed by the dictation at once gives its cue at once | E2E `.feature`, latency from the command's end sample |
| Start followed by the dictation at once gives its cue at once | E2E `.feature`, latency from the command's end sample |
| Start said on its own gives its cue at once | E2E `.feature`, latency from the command's end sample |
| A chain ending in start gives its cue at once | E2E `.feature`, latency from the command's end sample |
| Restart restart followed by the new dictation at once gives its cue at once | E2E `.feature`, latency from the command's end sample |
| Restart said on its own gives its cue at once | E2E `.feature`, latency from the command's end sample |
| The user's recorded starts give their cue at once | E2E `.feature`, user's recordings |
| A faster start keeps the dictation where the command's words end | existing E2E scenarios |
| A faster start adds no false start | existing E2E scenarios + `placed_commands_it` |
| A voice start logs its timing | component `.feature` |
| The timing stays right after microphone audio was lost | unit + component `.feature` |
| A hotkey start logs no voice start timing | component `.feature` |
| D2–D4 mechanics | unit + `engine_it` |
