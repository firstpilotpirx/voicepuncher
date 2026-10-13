# Proposal

## Why

With voice control on, the hotkey no longer records: since 0.7.0 it only turns listening on and off. The user wants every push-to-talk feature to work as before, with voice control added on top: the hotkey starts and stops a dictation (toggle or hold, as configured), Escape cancels it, and every dictation — started by the hotkey or by voice — shows the overlay and plays the start, stop and cancel sounds. In the user's first manual run of 0.8.0 a dictation started by voice showed no overlay and played no start sound.

## What Changes

- With voice control on, the hotkey records exactly as with voice control off: a press starts a dictation, a press (toggle) or the release (hold) ends it and pastes its text, Escape cancels it.
- Listening stays on for as long as the app runs with voice control on; the hotkey no longer turns it off. The way to dictate without voice commands is to run without voice control.
- The hotkey and the voice commands drive the same dictation: "stop dictation" or "cancel dictation" ends a dictation the hotkey started, and the hotkey ends one "start dictation" started. While the hotkey's dictation runs, the service listens for the dictation phrases.
- A dictation started by voice shows the overlay and plays the start sound, like one started by the hotkey; ending or cancelling it plays the stop or cancel sound and the overlay follows as in push-to-talk. The cause of the missing overlay and sound in the user's run is found and fixed, with a test that fails without the fix.

- One-word short forms, asked for by the user after the proposal: "enter" for "press enter", "clear" for "clear all text", "start" for "start dictation", "stop" for "stop dictation", each with or without "please". "cancel dictation" keeps no short form. A Russian "стоп" said alone is no longer counted among the phrases that must give no command, since whisper may hear it as "stop".

## Capabilities

### Modified Capabilities

- `voice-control`: the hotkey records as in push-to-talk while voice control is on; listening stays on and is no longer toggled by the hotkey; a dictation started either way can be ended either way; every dictation shows the overlay and plays the push-to-talk sounds; one-word short forms of four commands.

## Impact

- `app/src/core/application/pipeline/voice_control.rs`: hotkey signals go through the push-to-talk step; the phrase set follows the dictation whatever started it; the listening toggle goes away.
- `app/src/main.rs` and the overlay/sound adapters only if the missing overlay or sound has its cause there.
- Tests: pipeline unit tests, component `.feature` scenarios in `app/tests/features/component/voice_control.feature` (hotkey dictation with voice control on, mixed hotkey/voice start and stop, overlay and sounds of a voice dictation); obsolete listening-toggle scenarios removed.
- `command-spotting/src/matcher.rs`: the short forms as aliases.
- E2E: short forms with `say`; "стоп" leaves the short Russian phrases scenario.
- Docs: README voice control section, glossary if "listening" changes meaning.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:** the component scenarios of "Push-to-talk works with voice control on or off" and the hotkey/voice rules in `voice_control.feature` pass, with the pipeline unit tests for toggle and hold mode; the new scenarios pass; `just check` and `just full-check` (bar the known chunk-seam failure) pass; the user's manual run shows the overlay and hears the sounds for a hotkey dictation and for a voice dictation, and the hotkey and voice commands end each other's dictations.
- **Out of scope:** new voice commands; a hotkey or command to pause listening.

## Release

MINOR — the hotkey's behavior with voice control on changes; target version **0.9.0**.
