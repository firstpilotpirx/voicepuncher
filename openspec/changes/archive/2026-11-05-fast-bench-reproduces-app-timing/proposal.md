## Why

While fixing `fix-missed-dictation-start-stop-commands` (released as v0.46.3), the lone "restart" cue was measured three ways and the numbers disagreed: `just streaming-bench --fast`, the service fed in real time and the `e2e-cues` stage. The fast bench hands the audio over as fast as the session takes it, so each recognizer's decode reaches the session at an arbitrary point of the stream: the gate's pause and the decoded words meet at other moments than in the service, and the bench prints no delay at all in that pace. Twelve `kit verify` runs were needed where a trusted bench would have found the same by hand.

## What Changes

- `just streaming-bench --fast` replays the service's clock: the app's 20 ms frames, the gate, the flush and the close as fed in real time, and each recognizer's decode made visible to the session at the stream time it would have ended in the service on this machine (its measured decode time, queued behind the decode before it). The delay it prints for a command matches the real-time path within the margin the spec states.
- The bench's output names what the replay does not reproduce: a load other than its own, the service's poll and wake, the app's audio path and its syncs, the 20 ms frame granularity.
- Tests: the mechanism on scripted recognizers (unit), and the agreement of the two paces on the lone "restart" of the cues scenario and on a sample of the user's recordings (E2E, under the machine lock).

Current stage: PoC. Target stage: PoC.
Scope: the streaming bench of `command-spotting/` (`--fast`, `--recorded`), the session's delivery of the recognizers' decodes, its report.
Acceptance bar: the bench's fast cue for the lone "restart" of the cues scenario and for the user's sessions 1–6 matches the real-time path within the margin the spec states (the sessions measured by hand, recorded in the design); the bench names what it does not reproduce; `just test` and `just check` pass.
Out of scope: the E2E cues stage, the gate, the close's logic, the service protocol, the bench's real-time pace.
Interface: CLI (the bench's output).

## Capabilities

### Modified Capabilities

- `voice-control`: the fast streaming bench reproduces the service's timing or names what it cannot.

## Impact

- `command-spotting/src/bench/`, `command-spotting/src/streaming/session/` (the decoder's delivery), `command-spotting/src/bin/streaming_bench.rs`, `e2e-test/tests/features/voice_control.feature` and its streaming steps, `docs/ubiquitous-language.md`.

## Open points

- Capability: `voice-control`, where the bench's accuracy requirement already lives.
- Margin between the fast and the real-time cue: 0.1 s (frame granularity, the service's poll, decode jitter), confirmed by the measurement in the design.
- The server's wake and poll are not replayed, only named in the output.

## Release

SemVer: no release — a developer tool changes, no behavior the user sees.
Target version: 0.46.3 (unchanged).
