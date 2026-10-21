## Why

The audit of 2026-10-22 traced the open "whisper > 10 s stall" of voice control to three decisions in `command-spotting/` (verified in the code): a decoding timeout within the budget returns `Ok(Heard::default())` (`src/confirm.rs:163–169`, "still decoding, nothing heard"), so the second attempt of a segment (`worker.rs:37–43`) never fires and the command is lost silently; one confirmation thread per connection and an outbox that releases a segment's events only from the head of the queue (`outbox.rs:109–133`) make one slow segment delay every later segment, tail and unheld — head-of-line blocking that `Health` (`health.rs`, `STUCK_AFTER` 10 s) does not see because the processing thread never blocks; the input channel (`server.rs:93–120`, lag only logged past 1 s) and the job channel (`worker.rs:185–211`) are unbounded and `submit` swallows a send error, so a lasting backlog grows memory with no signal to the client. The protocol has no version (`protocol.rs:15`, `#[serde(untagged)]`, unreadable parse errors), malformed JSON drops the connection with no close reason (`server.rs:126`), `samples_from_bytes` drops trailing bytes silently, and `SpottedChecks::found_nothing` (`engine/spotted.rs:54–61`) overwrites its single slot. In `copy-pool`, `discover` probes synchronously inside `UtteranceTranscriber::new` (`pool.rs:106–125`, up to 1 s) and the API is wider than its use (`lease_avoiding`, `has_copies`, `snapshot`, `is_same`, `place` unused).

## What Changes

- A timed-out confirmation is a service failure: it is retried on another copy within the segment's budget, and a segment that gets no answer ends as "confirmation failed" in the events, never as "nothing heard"; the app shows the voice control failure notice.
- Confirmation lag is health: the age of the oldest unanswered job and the outbox depth are part of `Health`; past a threshold the service reports `stalled` to the app (a protocol event), and the app's supervision restarts the copy as `service-reliability-review` defines.
- Bounded queues: the input and job channels have a capacity; past it the oldest audio is dropped and the client is told (`Dropped { frames }` event), so memory is bounded and the outage is visible.
- Protocol: a `version` field on the handshake, a tagged `AppMessage`, a close reason on malformed input, an error for a byte count that is not whole samples.
- `found_nothing` keeps every check seen while waiting.
- `copy-pool`: discovery is lazy or asynchronous (the constructor returns at once); the unused API is removed.

Current stage: PoC. Target stage: MVP.
Scope: `command-spotting/src/{confirm,worker,outbox,health,server,protocol}.rs`, `engine/spotted.rs`, `copy-pool/`, the app's websocket client (`version`, `stalled`, `Dropped`), the `@resilience` E2E and `engine_it`.
Acceptance bar: with one copy paused mid-command the command is confirmed on another copy within the budget; with all copies paused the app shows the notice within `STUCK_AFTER`; a flood of audio past the capacity yields `Dropped` events and bounded memory (test); a malformed message yields a close reason.
Out of scope: the matching special cases (`tame-confirming-special-cases`); service copies and restarts (`service-reliability-review`, which this change waits for).

## Capabilities

### Modified Capabilities

- `voice-control`: a timed-out confirmation is retried then reported; stalls and drops are visible.
- `local-services`: confirmation lag is part of the command spotting service's health.

## Impact

- `command-spotting/src/**`, `copy-pool/src/**`, `app/src/adapters/secondary/commandspotting/websocket/**`, `e2e-test/tests/features/voice_control.feature` (`@resilience`), `openspec/specs/{voice-control,local-services}`.

## Open points

- Capacity of the input queue: 10 s of audio (160 000 samples) proposed.
- Whether the protocol version bump is a MINOR for the app (a client/server mismatch refuses the handshake); proposed: yes, MINOR.

## Release

SemVer: MINOR — protocol change (versioned handshake, new events).
Target version: next MINOR above `main` at merge (0.37.0 if `service-reliability-review` lands 0.36.0).
