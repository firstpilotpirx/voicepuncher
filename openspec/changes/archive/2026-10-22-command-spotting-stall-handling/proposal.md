## Why

The audit of 2026-10-22 traced the open "whisper > 10 s stall" of voice control to `command-spotting/`: a copy still decoding past its budget ends the confirmation as "nothing heard" (`confirm.rs`), so the segment's second attempt never runs and the command is lost silently; a confirmation thread that hangs is invisible to `Health`, which only watches the audio loop; the input and job channels are unbounded and a failed `submit` is swallowed; the protocol has no version, malformed JSON drops the connection without a reason, trailing bytes of an audio frame are dropped silently; `SpottedChecks::found_nothing` overwrites its single slot; `CopyPool` probes every copy synchronously in its constructor and keeps `is_same`, used only by tests.

## What Changes

- A copy that does not answer within its decoding budget hands the confirmation to another copy while the deadline lasts; a segment whose confirmation gets no answer is sent to the app as a failed confirmation, and the app shows that voice commands are unavailable.
- A confirmation running far past its attempts makes the service unhealthy (`/health` 503) and tells the app (`stalled`), which moves to another copy as after a loss.
- Bounded queues: once audio has waited past 10 s, new audio is dropped and the app is told (`dropped`), and treats the stream as lost; past a few waiting confirmations per thread, a new one fails at once.
- Protocol 2: a `hello` with the version opens every connection; messages are tagged; malformed input and partial samples close the connection with a reason.
- `found_nothing` keeps every waiting check.
- `copy-pool`: the constructor returns at once, the first request waits for the first probe round; `is_same` is removed.

Current stage: PoC. Target stage: MVP.
Scope: `command-spotting/src/**`, `copy-pool/src/**`, the app's command spotting adapter and voice control pipeline, their tests.
Acceptance bar: one copy stalled mid-command → confirmed on another copy; every copy stalled → the app shows the notice within 10 s; a flood past the capacity yields `dropped` and bounded memory; a malformed message and a wrong version yield a close reason (all by tests).
Out of scope: matching special cases (`tame-confirming-special-cases`); service copies and restarts (`service-reliability-review`, landed).

## Capabilities

### Modified Capabilities

- `voice-control`: a failed confirmation is retried, then reported; drops are visible; the protocol is versioned.
- `local-services`: a hung confirmation is part of the service's health; copy discovery does not block.

## Impact

`command-spotting/src/**`, `copy-pool/src/**`, `app/src/adapters/secondary/commandspotting/websocket/**`, `app/src/core/{domain,application/pipeline}`, tests.

## Release

SemVer: MINOR — protocol change and new behavior. Target version: 0.37.0.
