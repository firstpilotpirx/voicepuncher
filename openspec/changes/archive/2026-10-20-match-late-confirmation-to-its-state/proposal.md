## Why

`commandspotting_websocket_it` once heard "start dictation … stop dictation" as a restart and a stop. The confirmation of "start dictation" came back after the session had moved to the dictating phrases, and was matched against those: the start became a restart. A command must be carried out as it was said, however slowly its confirmation comes back.

## What Changes

- A command confirmation is matched against the phrases of the state in which its words were heard together with those of the current state, the closest phrase winning, so a state change before the confirmation comes back never turns one command into another.
- "start dictation" said in standby is carried out as a start, never as a restart, whatever the confirmation delay; a "start" said right after a slowly confirmed "stop" stays a start.

Current stage: PoC. Target stage: PoC.
Scope: matching of late confirmations in the command spotting service (`command-spotting/`).
Acceptance bar: a test with delayed confirmations, where the state changes before a confirmation returns, gets the command that was said; `just full-check` passes.
Out of scope: the start-cue latency (speed-up-short-start-cue), confirmation timeouts, the phrase sets themselves.

## Capabilities

### Modified Capabilities

- `voice-control`: a late confirmation is matched against the phrases of the state in which the words were heard and of the current state.

## Impact

- `command-spotting/src/engine.rs` and the confirmation path it drives
- `command-spotting/src/matcher.rs`; `command-spotting/tests/placed_commands_it.rs` (delayed-hearing tests)

## Open points

- A command the current state does not accept — settled: reported as said; the app ignores it.

## Release

SemVer: PATCH — bug fix in command matching.
Target version: 0.31.3.
