## Why

While fixing the flaky tests (0.24.1), `commandspotting_websocket_it` returned `[None, Some(RestartDictation), Some(StopDictation)]` for "start dictation … stop dictation". The confirmation of "start dictation" came back late, after the session had moved from standby to dictating, and the engine matched it against the dictating phrases, not those it was said under: the start was carried out as a restart.

The test now waits for its audio, but a slow confirmation can still turn one command into another. A command must be carried out as it was said, however slowly its confirmation comes back.

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
- `command-spotting/tests/placed_commands_it.rs` (delayed-hearing tests), `app/tests/commandspotting_websocket_it.rs`

## Open points

- Whether a confirmation whose command no longer fits the current state is dropped or carried out — settled: reported as said; the app ignores a command its state does not accept.
- Target version — proposed 0.24.3, the PATCH above 0.24.2 on `main`.

## Release

SemVer: PATCH — bug fix in command matching.
Target version: 0.24.3.
