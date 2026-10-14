# Proposal

## Why

Since `survive-service-stalls` (0.13.0), listening stays on while the command spotting service is away: the dictation keeps recording, the app reconnects, and the speech of the last seconds is sent again once the service is back. The only sign of the outage is a sound: the listening-off sound on loss and the listening-on sound on return.

That sound says the wrong thing. Listening is not off; only the voice commands do not work for a while. A short outage, such as the restart after a crash, is not even noticeable: the replay carries out a command said during it. The user still hears "off" and "on" and wonders what they did. A long outage, when the service does not come back, is the case that needs a signal, and a sound played once is easy to miss.

## What Changes

- While the command spotting service is away, the overlay widget shows that voice commands do not work now, for as long as the outage lasts. It shows this during a dictation and in standby, appearing on its own if it is hidden.
- When the service is back, the overlay returns to what it showed before (recording, or hidden).
- No signal for an outage short enough that the replay covers it: the overlay shows the outage only once it has lasted a few seconds (design picks the threshold from the replay window).
- The listening-off and listening-on sounds no longer play on loss and return. Listening-on still plays when the app starts.
- The log keeps the loss, each reconnect attempt and the return, as today.

## Capabilities

### Modified Capabilities

- `voice-control`: the loss and the return of the command spotting service are shown on the overlay instead of played as listening sounds; a short outage shows nothing.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: the overlay state for "voice commands unavailable", its threshold, removing the loss and return sounds, tests at component and E2E level, the glossary term.
- Acceptance bar: with the command spotting service killed and restarted by the scripts within a few seconds, nothing is shown and no sound plays, and a command said meanwhile still works; with the service kept away for longer, the overlay shows that voice commands do not work until it is back, during a dictation and in standby; the user's manual run confirms the overlay is readable and does not take focus.

## Out of scope

- Notices for other failures (speech recognition, text refinement, permissions): `notify-service-failures`. This change may share its overlay surface; whichever lands later builds on the other.
- Any change to reconnecting, the replay or the service scripts.

## Release

MINOR (behavior change of an existing capability, new overlay state), the next minor version after the one in `app/Cargo.toml` when it is implemented.

## Impact

- `app/src/core/application/pipeline/voice_control.rs`: the loss and the return drive the overlay (after the threshold) instead of the sounds.
- `VisualIndicator` port and the macOS overlay adapter (`app/src/adapters/secondary/visualfeedback/`): a new state for "voice commands unavailable".
- Tests: component scenarios in `voice_control.feature` (overlay events of the doubles), E2E outage scenarios in `e2e-test/tests/features/voice_control.feature`, which today detect the loss and the return by the sounds.
- `docs/ubiquitous-language.md`: the overlay state; the "Away" entry.
- Manual run by the user (overlay).
