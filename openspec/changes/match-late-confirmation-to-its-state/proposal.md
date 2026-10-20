## Why

While fixing the flaky tests (fix-flaky-tests, released as 0.24.1), `commandspotting_websocket_it` returned `[None, Some(RestartDictation), Some(StopDictation)]` for speech that said "start dictation … stop dictation". Under load the speech recognition service for commands answered the confirmation of "start dictation" late. By then the session had already moved from standby to dictating, and the command spotting engine matched the confirmed text against the phrase set of the current state (dictating), not the state in which the words were heard (standby). So "start dictation" was carried out as "restart dictation".

The test was fixed to wait for its audio, but the product behavior stays: a slow confirmation can turn one command into another. The user's rule is that a command is carried out as it was said, however slowly its confirmation comes back.

## What Changes

- A command confirmation is matched against the phrases of the state in which its words were heard, even when the state changed before the confirmation came back.
- "start dictation" said in standby is carried out as a start, never as a restart, whatever the confirmation delay.

Current stage: PoC. Target stage: PoC.
Scope: matching of late confirmations in the command spotting service (`command-spotting/`).
Acceptance bar: a test with delayed confirmations, where the state changes before a confirmation returns, gets the command that was said; `just full-check` passes.
Out of scope: the start-cue latency (speed-up-short-start-cue), confirmation timeouts, the phrase sets themselves.

## Capabilities

### Modified Capabilities

- `voice-control`: a late confirmation is matched against the phrases of the state in which the words were heard.

## Impact

- `command-spotting/src/engine.rs` and the confirmation path it drives
- `command-spotting/tests/placed_commands_it.rs` (delayed-hearing tests), `app/tests/commandspotting_websocket_it.rs`

## Open points

- Whether a confirmation whose command no longer fits the current state (a start that arrives while already dictating) is dropped or carried out — proposed: carried out as said only when it still makes sense, else dropped; settle in design.
- Target version — proposed 0.24.3, the PATCH above 0.24.2 on `main`.

## Release

SemVer: PATCH — bug fix in command matching.
Target version: 0.24.3.
