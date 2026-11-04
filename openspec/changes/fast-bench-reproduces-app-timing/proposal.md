## Why

While fixing `fix-missed-dictation-start-stop-commands` (archived, released as v0.46.2), the lone "restart" cue was measured three ways and the numbers disagreed: `just streaming-bench --fast` on the user's recordings, the real-time runs of the service and the `e2e-cues` stage. The fast bench feeds the audio faster than real time, so the gate's windows, the flush at the gate's pause, the close silence and the recognizers' decode moments meet the audio at other points than in the app, which sends 20 ms frames as captured: a cue the bench measured by hand did not match what the E2E stage then measured, and twelve `kit verify` runs were needed where fewer hand runs would have found the same.

## What Changes

- `just streaming-bench --fast` reproduces the app's timing: the app's frame size (320 samples), the gate's windows, the flush and the close silence at the same moments as in the service fed in real time, so the cue it prints matches the `e2e-cues` stage for the same audio.
- Where the fast bench cannot reproduce a timing (decode moments under load, the wake of the server by a decode), it reports that in its output beside the number, so a hand measurement is never taken for the stage's.
- A test compares the bench's cue for one recorded command with the cue the real-time path gives for the same audio, within a stated margin.

Current stage: PoC. Target stage: PoC.
Scope: the streaming bench of `command-spotting/` (`--fast` and `--recorded`), its frame feeding and its report.
Acceptance bar: the bench's cue for the lone "restart" of the cues scenario and for sessions 1–6 matches the real-time path within the margin the design states; the bench names what it cannot reproduce; `just test` and `just check` pass.
Out of scope: the E2E cues stage, the gate, the session's closing logic.
Interface: CLI (the bench's output).

## Capabilities

### Modified Capabilities

- `command-spotting-bench`: the fast bench reproduces the app's frame timing or names what it cannot.

## Impact

- `command-spotting/src/bench/` (or wherever `streaming-bench` lives), `justfile` recipe `streaming-bench`, a test in `command-spotting/tests/it/`.

## Open points

- The capability name: the bench may live under another spec (`command-spotting` or `voice-control`); the specs stage names the right one.
- The margin between the bench's cue and the real-time path: proposed 50 ms.
- Whether the bench also replays the server's wake on a decode, or only the frame timing: proposed frame timing only, the rest reported.

## Release

SemVer: no release — a developer tool changes, no behavior the user sees.
Target version: 0.46.2 (unchanged).
