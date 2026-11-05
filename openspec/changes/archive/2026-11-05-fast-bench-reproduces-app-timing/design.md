# Design

## Context

See proposal.md § Why. Each recognizer decodes on a thread of its own; at each 20 ms frame the session collects the updates that arrived and the utterance holds or releases them against the gate's state. In the fast pace the feeder runs ahead of the threads, so an update meets the gate at an arbitrary frame.

## Goals / Non-Goals

**Goals:** the fast pace yields the real-time delays within 0.1 s; the real-time pace and the service keep their timing.

**Non-Goals:** replaying the service's poll and wake, the app's audio path and its syncs, or a load other than the bench's own; a faster real-time pace.

## Decisions

**Replayed delivery in the decoder, chosen by the session's configuration.** The decoder gets a delivery mode: as decoded (the service and the real-time pace) or replayed (the fast pace). Replayed, every job carries the stream time it was fed at; the update becomes visible at `max(fed at, the previous update's visible time) + decode time`, the queue of a thread slower than the audio. At each frame the collect blocks until the thread decoded everything fed, then releases only the updates visible by the frame's end; the progress the utterance reads follows the released updates; a sync releases everything. In the decoder, so the utterance's hold, release, handover and close see the same updates as in real time. Rejected: releasing every decode in the frame it was fed (earlier than the service); stamping the delay from the thread's clock (the gate's pause and the words still meet at other frames).

**The fast delay is the stream position.** From the end of the words to the end of the frame whose collect reported the command, in stream seconds, 20 ms granularity; the real-time pace keeps the wall clock.

**A footer names what is not replayed.** After the summaries the fast pace prints what the replay reproduces and what it does not (§ Non-Goals, time within a frame).

No new dependency, no protocol change.

## Risks / Trade-offs

- [The decode times of two runs differ: the GPU, the other recognizer's thread] → the margin of 0.1 s covers the jitter measured below; the E2E runs both paces under the e2e lock.
- [Blocking at every frame slows the fast pace] → still faster than real time by the recognizers' speed; the decoded text does not depend on when it is seen.
- [`utterance.rs` holds 293 of its 300 code lines] → the stamping lives in the decoder.

## Measurements

2026-11-05, `just streaming-bench --recorded ~/.voicepuncher/voice-samples --word <word>` on sessions 1–6 (median / p90), and the E2E scenarios' `paces:` lines per clip:

| Clips | Real time | Fast |
|---|---|---|
| "start", 5 timed | 446 / 1228 ms | 420 / 1200 ms |
| "stop", 8 timed | 394 / 767 ms | 390 / 760 ms |
| lone "restart" after 20 s of dictation | 220 ms | 292 ms |
| session-1/25 "enter" | 619 ms | 610 ms |
| session-2/8 "restart" | 912 ms | 910 ms |
| session-3/3 "cancel" | 1392 ms | 1390 ms |

The largest difference, 72 ms, is within 0.1 s: the spec keeps the margin. What is heard differed on 2 of 15 "stop" clips (a doubled "stop" on session-4/5, the "clear" after "stop" missed on session-5/10, fast pace); one run each, not yet told from the recognizers' run-to-run variance.

## Test levels

| Spec scenario | Level |
|---|---|
| A lone restart gives the same delay in both paces | E2E (`voice_control.feature`, `@regression`) |
| The user's recordings give the same delays in both paces | E2E (`@voice_samples @regression`, a sample of 3) |
| A decode is seen when it would have ended | unit (session, a scripted recognizer of 30 ms per decode) |
| The fast report names what it does not replay | unit (bench) |

## Interface mock

https://claude.ai/artifact/KZ1rxnASbmz2H4mEEMwW4y — `just streaming-bench --recorded --fast` with its footer lines, in a light and a dark terminal.
