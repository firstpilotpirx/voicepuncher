## Why

After "start start" the overlay and start cue come late: median 837 ms, one in ten over 1.2 s (239 starts in the user's log), against a goal of 100–300 ms. The time goes to waiting for a pause, the Whisper confirmation and the app opening a second microphone.

## What Changes

- The dictation records from the microphone voice control already holds; the overlay shows at once.
- The start cue plays from a preloaded sound inside the app.
- The overlay's windows are built once, rebuilt on a screen change.
- The start log line tells when the cue sounded and the overlay was drawn.
- The command spotting service wakes on answers instead of polling.
- A pause is recognized in a noisy room without the slow fallback detector.
- A doubled command ("start start", "stop stop") is carried out as soon as its second word is heard, with no pause.
- A setting "Confirm commands with Whisper" (on by default): off, commands run on the keyword spotter alone, while Whisper still hears each one afterwards and the log and command history record whether it agreed.
- A step that adds false starts or loses accuracy on the user's recordings is dropped, with its measurement reported.

Current stage: PoC. Target stage: PoC.
Scope: voice start and doubled command latency, from the end of the words to the overlay and cue.
Acceptance bar: median voice start at most 350 ms, one in ten at most 500 ms, on the user's machine; no more false starts and no worse accuracy on the user's recordings.
Out of scope: hotkey start latency; transcription speed.

## Capabilities

### Modified Capabilities

- `voice-control`: faster starts, doubled commands without a pause, a pause recognized in noise, commands without confirmation, a finer start log line.
- `visual-feedback`: the overlay appears without rebuilding its windows.
- `settings-window`: the confirmation switch.

## Impact

`command-spotting/`, the app's recorder, sounds, overlay, pipeline and settings, the `e2e-cues` bounds; builds on `log-voice-command-history`.

## Release

SemVer: MINOR. Target version: 0.40.0 (next free minor).
