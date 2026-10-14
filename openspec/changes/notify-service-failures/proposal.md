# Proposal

## Why

When something the user relies on fails, the app stays silent. A voice command is said and nothing starts because the speech recognition service is not running; a dictation is recorded and then dropped because the transcript could not be produced; voice control stops listening because the command spotting service is unreachable. In every case the overlay never appears or simply hides, no sound plays, and the only trace is a line in the terminal log the user is not looking at. The user cannot tell a failure from "it did not hear me", so they repeat the command, wait, or restart the app without knowing what broke.

## What Changes

- When an action the user started cannot be completed because of a failure, the app tells the user, right away and on screen, that it failed and what failed: which local service is unreachable or not answering (speech recognition, text refinement, command spotting), or which permission or device is missing (microphone, accessibility, input monitoring).
- The notice names the failing part in plain words and, when there is a known remedy, says it (for example, start the local services).
- The notice appears for a dictation started by the hotkey and one started by voice, for a voice command that cannot be carried out, and when voice control stops listening.
- A dictation whose transcript cannot be produced is no longer dropped silently: the user is told the dictation was lost and why.
- When text refinement fails and the raw transcript is pasted instead (as today), the user is told the text was pasted without refinement.
- The notice goes away on its own, does not steal focus and does not block the next dictation.
- The same failure repeated in a row does not flood the screen with notices.
- The log keeps the technical detail, as today.

## Capabilities

### New Capabilities

- `failure-notices`: the user is told on screen what failed whenever an action they started cannot be completed.

### Modified Capabilities

- `push-to-talk`, `voice-control`, `speech-recognition`, `text-refinement`: the requirements that today drop a dictation, ignore a command or fall back silently now also notify the user.

## Scope

- Current stage: PoC. Target stage: PoC.
- In scope: failures of the speech recognition, text refinement and command spotting services, a missing microphone or TCC permission, and a failed paste, during push-to-talk and voice control.
- Acceptance bar: with the speech recognition service stopped, a dictation started by the hotkey or by voice ends with an on-screen notice that names speech recognition as the cause within 2 s of the stop, instead of nothing; with the command spotting service stopped, starting voice control shows a notice naming command spotting; with the text refinement service stopped, the raw transcript is pasted and a notice says refinement was skipped; no failure in scope leaves the user with no on-screen sign.

## Out of scope

- Starting, restarting or supervising the local services from the app.
- Retrying a failed dictation or keeping its audio for a later retry.
- macOS Notification Center integration beyond what design picks as the notice surface.
- Failures in file transcription mode (it already reports errors in the terminal).
- Changing the startup checks; they already fail fast in the terminal.

## Release

MINOR (new capability), next minor version after the current one in `app/Cargo.toml` (0.10.1 → 0.11.0 at the time of writing).

## Impact

- The pipeline and voice control report each failure in scope to a new notice signal instead of only logging it; the failure carries which part failed.
- A surface to show the notice: the overlay (a new failure state with a short message) or a macOS notification — design decides.
- `docs/ubiquitous-language.md`: a term for the failure notice; the `VisualIndicator` definition if the overlay gains a failure signal.
- The overlay is also changed by `show-processing-stages`, `show-live-transcript` and `confirm-voice-commands`; whichever lands later builds on the others.
- Manual run by the user (overlay, recording, permissions).
