# Design

## Context

See proposal.md § Why. Each recognizer decodes on a thread of its own after every job; at each 20 ms frame the session collects the updates that arrived and the utterance holds or releases them against the gate's state. In the fast pace the feeder runs ahead of the threads, so an update meets the gate at an arbitrary frame.

## Goals / Non-Goals

**Goals:** the fast pace yields the real-time delays within 0.1 s; the real-time pace and the service keep their timing.

**Non-Goals:** replaying the service's poll and wake, the app's audio path and its syncs, or a load other than the bench's own; a faster real-time pace.

## Decisions

**Replayed delivery in the decoder, chosen by the session's configuration.** The decoder gets a delivery mode: as decoded (today: the service and the real-time pace) or replayed (the fast pace). In the replayed mode every job carries the stream time it was fed at; the thread answers with that time and its decode time; the update becomes visible at `max(fed at, the previous update's visible time) + decode time`, the queue of a thread slower than the audio. At each frame the collect blocks until the thread decoded everything fed, then releases only the updates visible at the frame's end; the progress the utterance reads (decoded all, finished) follows the released updates. A sync releases everything, as the app's sync waits for the recognizers. In the decoder, so the utterance's hold and release, handover and close see the same updates as in real time. Rejected: catching up at every frame and releasing at once (a decode ends in the frame it was fed, earlier than the service); a free-running feeder stamping the delay from the thread's clock (the gate's pause and the words still meet at other frames).

**The fast delay is the stream position.** In the fast pace a command's delay counts from the end of its words to the end of the frame whose collect reported it, in stream seconds, with 20 ms granularity; the real-time pace keeps the wall clock. Outcome and summary print both alike.

**A footer names what is not replayed.** After the summaries the fast pace prints what the replay reproduces (the decode times measured on this machine; the frames, gate, flush and close as in real time) and what it does not (a load other than the bench's own, the service's poll and wake, the app's audio path and its syncs, time within a frame). The real-time pace prints none.

No new dependency, no protocol change.

## Risks / Trade-offs

- [The decode times of two runs differ: the GPU, the other recognizer's thread] → the margin of 0.1 s covers the jitter measured below; the E2E runs both paces on the machine alone, under the e2e lock.
- [Blocking at every frame slows the fast pace] → it still runs faster than real time by the recognizers' speed; the accuracy scenarios keep their results, as the decoded text does not depend on when it is seen.
- [`utterance.rs` holds 293 of its 300 code lines] → the stamping lives in the decoder; the utterance only passes `now` on.

## Measurements

Filled at apply: `just streaming-bench --recorded` by word in real time and `--fast`, the lone "restart" in both paces; median, p90 and the largest per-clip difference on sessions 1–6.

## Test levels

| Spec scenario | Level |
|---|---|
| A lone restart gives the same delay in both paces | E2E `.feature` (`voice_control.feature`, `@regression`) |
| The user's recordings give the same delays in both paces | E2E `.feature` (`@voice_samples @regression`, a sample of the recordings) |
| A decode is seen when it would have ended | unit (session tests, a scripted recognizer taking 30 ms per decode) |
| The fast report names what it does not replay | unit (bench) |

## Interface mock

https://claude.ai/artifact/KZ1rxnASbmz2H4mEEMwW4y — the output of `just streaming-bench --recorded --fast` with its footer lines, in a light and a dark terminal.
