# Design

## Context

See proposal.md § Known and suspected causes. The keyword spotter reports "start" ≈0.25 s after its words, but a running utterance is confirmed only by timed checks. The lead-in already comes from the last 10 s of streamed audio, so only the cue moves.

## Decisions

**D1 — Measure first, with the timing log.** The timing log (D5) and an E2E latency harness land first; today's numbers per placement go into this file before any fix. The conditional part of D6 lands only if the numbers still miss the bar. Rejected: fixing from estimates.

**D2 — A spotted start triggers a check.** When the spotter reports "start dictation" or "restart dictation" in a running utterance, the service asks a check once 0.35 s of audio follows the phrase, so the next word has begun ("start stop" must not start). It uses the head clip, or the last 2.5 s of a longer utterance, and the head and window matchers; "start" heard alone is taken only from a second check 0.3 s later. While another check waits, it is asked when that one answers: at most one check waits (`local-services`). The spotter gets trigger-only keywords for a short "start" and "restart": they only ask this check and are never reported as phrases, so what counts as a command stays with the confirmation. Timed checks stay as a fallback. Rejected: starting on the spotter's report alone (false starts). Load: one ≤2.5 s confirmation per spotted start.

**D3 — The head check takes doubled starts and chains.** It also applies the "anywhere" matcher, so "start start" and "enter start" count at once.

**D4 — Segment confirmations do not wait behind checks.** Each connection gets a second confirmation thread for checks; segments, unheld phrases, tails and re-hearings keep the first. At most two requests per connection are in flight, for three speech recognition copies. Rejected: cancelling a running check (an HTTP request cannot be withdrawn).

**D5 — The timing log.** The app's heard audio records when each streamed block was captured, so a stream position maps to its capture time even after audio was lost. Command and phrase events carry an optional `timing`: the stream position at which their confirmation was asked and its duration. At the cue the app logs `voice start: total=… hear=… confirm=… deliver=… cue=…` in ms, from the capture time of the command's end.

**D6 — "restart" does not wait for the dropped dictation.** Cancel stops the recorder without flushing and plays the cues at once; the new live session takes the transcriber back from the old worker before its first chunk. Only if D1's numbers miss the bar: confirm a lone start during its pause, and show the overlay before the recorder opens.

## Risks / Trade-offs

- [A cut next word gives a false start] → the 0.35 s lead, the second check, every false-start scenario.
- [Extra load] → one check per spotted start; `just full-check` runs accuracy and seams with it.
- [Two threads reorder answers] → the outbox orders events by job.

## Dependencies

Waits for `show-live-transcript` (it reworks the live transcription worker, `pipeline.rs` and `voice_control.rs`) and `confirm-voice-commands` (it gives restart a sound of its own and splits the cancel and start paths). Once both are on `main`, D6 hands the transcriber over in their reworked paths, and restart's start cue is the restart sound they define.

## Test levels

| Scenario | Level |
|---|---|
| Start start followed by the dictation at once gives its cue at once | E2E `.feature`, latency from the command's end sample |
| Start followed by the dictation at once gives its cue at once | E2E `.feature`, latency from the command's end sample |
| Start said on its own gives its cue at once | E2E `.feature`, latency from the command's end sample |
| Restart restart followed by the new dictation at once gives its cue at once | E2E `.feature`, latency from the command's end sample |
| Restart said on its own gives its cue at once | E2E `.feature`, latency from the command's end sample |
| The user's recorded starts give their cue at once | E2E `.feature`, user's recordings |
| A faster start keeps the dictation where the command's words end | existing E2E scenarios |
| A faster start adds no false start | existing E2E scenarios + `placed_commands_it` |
| A voice start logs its timing | component `.feature` |
| The timing stays right after microphone audio was lost | unit + component `.feature` |
| A hotkey start logs no voice start timing | component `.feature` |
| D2–D4 mechanics | unit + `engine_it` |

## Measurements

Before the fix (task 1.8, 2026-10-18, commit `c7c6a15` plus harness fixes): the E2E scenarios of "A voice start gives its start cue within 0.7 s" (`just e2e-seams`), three runs for the synthesized voice and two for the user's recordings (`session-2`, starts from standby: "start", "start start", "start please"). Time from the sample that ends the command's words to the start (or restart) sound; nearest-rank p90. The restart cue comes after the recorder opens, so it includes the stand-in recorder's 0.3 s microphone opening.

| Placement | n | median | p90 | max |
|---|---|---|---|---|
| "start start" followed at once by speech (synthesized) | 3 | 3092 | 3109 | 3109 |
| "start" followed at once by speech (synthesized) | 3 | 1230 | 1240 | 1240 |
| "start" alone (synthesized) | 3 | 465 | 473 | 473 |
| "enter start" followed at once by speech (synthesized) | 0 of 3 heard | – | – | – |
| "restart restart" followed at once by speech, in a dictation (synthesized) | 1 of 3 heard | 1988 | 1988 | 1988 |
| "restart" alone after 20 s of dictation (synthesized) | 2 | 778 | 785 | 785 |
| The user's recorded starts alone | 9 | 741 | 962 | 962 |
| The user's recorded starts followed at once by speech | 9 | 1107 | 1766 | 1766 |

2 of the 12 recorded starts of the last run gave the cue within 0.7 s. "enter start" was never carried out (no Return, no start): the head check does not take a chain today (D2).

After group 2 (2026-10-18, commit `b0472d5`, tasks 2.1–2.12: head check with the anywhere matcher, spotted check, trigger-only short keywords for "start" and "restart", a second hearing of a short segment holding a trigger): the same scenarios, three runs; the user's recordings were cut short in one run (15 starts, each alone and followed by speech).

| Placement | n | median | p90 | max |
|---|---|---|---|---|
| "start start" followed at once by speech (synthesized) | 3 | 229 | 295 | 295 |
| "start" followed at once by speech (synthesized) | 3 | 535 | 565 | 565 |
| "start" alone (synthesized) | 3 | 534 | 552 | 552 |
| "enter start" followed at once by speech (synthesized) | 3 | 793 | 809 | 809 |
| "restart restart" followed at once by speech, in a dictation (synthesized) | 3 | 738 | 740 | 740 |
| "restart" alone after 20 s of dictation (synthesized) | 3 | 997 | 1000 | 1000 |
| The user's recorded starts alone | 15 | 851 | 1155 | 1243 |
| The user's recorded starts followed at once by speech | 15 | 781 | 1120 | 1150 |

11 of the 30 recorded starts gave the cue within 0.7 s. Before the trigger keywords (commit `2cf9664`) the spotted check was never asked: the keyword spotter knew only the full phrases and never reported a short "start" or "restart"; "start start" and "start" followed by speech took 1250 and 1241 ms. The triggers run on a keyword stream of their own and only ask the spotted check, since the spotter adds its own keywords (the phrases) to every stream and a trigger in the phrase stream would reset it before "dictation". "restart" alone: the segment confirmation hears the synthesized word as "Restocked." or "Restart." depending on where the voice activity detector starts the segment (0 of 6 heard on group 2 before the fix, 2 of 3 on `5502f30`, so flaky before group 2 as well); the restart trigger now gives such a segment the near-miss second hearing, heard in 3 of 3.

After groups 3–4 (2026-10-18, commit `b02688c`: checks on a confirmation thread of their own beside segments, a check answered after its utterance's segment dropped, "restart" without waiting for the dropped dictation, its cue before the recorder opens): the same scenarios, three runs, the user's recordings in full (12 starts per run).

| Placement | n | median | p90 | max |
|---|---|---|---|---|
| "start start" followed at once by speech (synthesized) | 3 | 221 | 225 | 225 |
| "start" followed at once by speech (synthesized) | 3 | 527 | 545 | 545 |
| "start" alone (synthesized) | 3 | 512 | 540 | 540 |
| "enter start" followed at once by speech (synthesized) | 3 | 791 | 801 | 801 |
| "restart restart" followed at once by speech, in a dictation (synthesized) | 3 | 472 | 493 | 493 |
| "restart" alone after 20 s of dictation (synthesized) | 3 | 531 | 535 | 535 |
| The user's recorded starts alone | 18 | 689 | 1122 | 1202 |
| The user's recorded starts followed at once by speech | 18 | 739 | 1092 | 1104 |

18 of the 36 recorded starts gave the cue within 0.7 s; "enter start" (0.79 s) and the recorded starts still miss the bar. "restart" no longer pays the dropped dictation's last request nor the stand-in recorder's 0.3 s opening (997 → 531 ms; "restart restart" 738 → 472 ms). With checks on their own thread, the first run of group 3 reported a check's command again after its utterance's segment (an extra "start" and a doubled "cancel" in the user's session-1, `voice_samples_it`, 2 of 2 runs; passed with group 3 reverted); a check answered after its segment is now dropped, and `just e2e-smoke` passes.

After group 5 (2026-10-18, commit `0305488`: the room after a timed command fed in real time, the start of an utterance searched 1.0 s in while "start" is listened for): the same scenarios, three runs, the user's recordings in full (12 starts per run).

| Placement | n | median | p90 | max |
|---|---|---|---|---|
| "start start" followed at once by speech (synthesized) | 3 | 213 | 219 | 219 |
| "start" followed at once by speech (synthesized) | 3 | 524 | 529 | 529 |
| "start" alone (synthesized) | 3 | 472 | 474 | 474 |
| "enter start" followed at once by speech (synthesized) | 3 | 785 | 812 | 812 |
| "restart restart" followed at once by speech, in a dictation (synthesized) | 3 | 466 | 475 | 475 |
| "restart" alone after 20 s of dictation (synthesized) | 3 | 458 | 579 | 579 |
| The user's recorded starts alone | 18 | 503 | 590 | 652 |
| The user's recorded starts followed at once by speech | 18 | 517 | 770 | 784 |

33 of the 36 recorded starts gave the cue within 0.7 s (11 of 12 in each run); the three misses are "start please" at 31.35 s followed by speech (762–784 ms): the early search 1.0 s in hears "start please" with nothing after it yet, which may still be "start please" alone, so the head check at 1.5 s takes it. "enter start" still misses the bar: the early search, both spotted checks and the windows hear the synthesized "Enter" as "and a" ("and a star to come", "and a start coffee"); only the head check at 1.5 s hears "Enter start coffee machine". Where the time went before group 5: the harness fed the room after a command a frame and then slept a frame, which ran about a quarter behind real time and delayed the pause that ends a lone start by 0.1–0.3 s (task 5.5); the keyword spotter fires the start trigger on 2 of 9 of the user's starts (at most 4 of 9 with any boost and threshold tried, while firing on "restart" in 9–12 other utterances), so a start followed by speech waited for the head check 1.5 s into the utterance (task 5.6). Hearing a short utterance during its pause (task 5.2) was tried and reverted: the clip heard 0.08 s into the pause is 0.03–0.07 s shorter than the segment's, and whisper then placed the first word of the user's "cancel dictation" (session-1, 41.25 s) 0.5 s earlier, so `voice_samples_it` failed in 2 of 2 runs. The cue stage (task 5.3) is not what holds the start: the synthesized "start start" gets its cue 213 ms after its words, hearing and confirmation included, and delivery takes 0–50 ms; the `cue=` of the app's `voice start:` line is about 315 ms only because it is logged after the stand-in recorder's 0.3 s opening, which follows the cue. `just e2e-smoke` passes; its first run failed once on "cancel dictation" heard as "Capsule dictation" by the segment confirmation in the synthesized hands-free session (dictating, where the early search is never asked) and passed on the unchanged rerun: a flake.

"enter start" is moved out of this change (follow-up `hear-enter-before-start`): its E2E scenario stays, tagged `@wip`, until that change hears "enter" in a short clip.
