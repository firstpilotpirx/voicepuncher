## Why

The voice commands that start and stop a dictation are periodically not recognized: the user says the start phrase or the stop phrase, and now and then nothing happens — no dictation starts, or the running dictation goes on. The failure is intermittent, not constant: the same phrases work most of the time, so the cause is not a missing phrase but something in the path from the microphone through the command spotting service (the Silero voice gate and the English and Russian sherpa-onnx streaming recognizers on port 8082) to the app's reaction to a reported command. A command that is missed at random makes hands-free use unreliable, and the user cannot tell whether the phrase was not heard, not spotted, spotted too late, or spotted and dropped by the app.

## What Changes

- Find out why the start and stop dictation phrases are sometimes not recognized: trace a missed command through the recorded evidence (the command history, the dictation archive, the app and service logs, the user's own recordings in `~/.voicepuncher/voice-samples/`) and name the stage that loses it (voice gate, recognizer, phrase matching, WebSocket delivery, the app's handling of a command during a running or ending dictation).
- Fix the cause so that the start and stop phrases are recognized reliably; a change that only tunes thresholds on synthesized speech is not enough — the fix is measured on the user's recordings, and the user is asked for a new recording session of the missed phrases when the existing recordings do not show the failure.
- Add a test that reproduces the missed command and stays red until the fix (§ TDD bug-fix rule), at the level where the loss happens.

Current stage: PoC. Target stage: PoC.
Scope: the recognition and delivery of the start and stop dictation commands, from the audio the command spotting service hears to the app's reaction; the diagnostics needed to see where a command is lost.
Acceptance bar: a reproducing test that failed before the fix passes; the start and stop phrases are recognized on every one of the user's recordings that reproduced the failure; `just check` passes; the user's manual run shows start and stop commands reacting every time over a session of repeated dictations.
Out of scope: the other voice commands, new command phrases or forms, changes to the dictation recognition model (whisper) or to the dictation text itself.

## Capabilities

### Modified Capabilities

- `voice-control`: the start and stop dictation commands are recognized reliably; a missed command leaves evidence of the stage that lost it.

## Impact

- `command-spotting/` (voice gate, streaming recognizers, phrase matching, command reporting), `voice-commands/` when the phrases or their mishearings need an entry, `app/` where the app handles a reported start or stop command, the `voice_samples` tests and `e2e-test/tests/features/voice_control.feature`.
- `just test`, `just check`, `just streaming-bench --recorded` on the user's recordings.

## Open points

- Which stage loses the command is unknown; the investigation decides it, and the design records the evidence before the fix. Proposed order: the command history and logs of a missed command first, then the user's recordings through `just streaming-bench --recorded`.
- Whether the loss is in the service (gate, recognizer, matching) or in the app (a command reported while a dictation is starting or stopping). Proposed: treat both as in scope until the evidence excludes one.
- Whether the existing recordings in `~/.voicepuncher/voice-samples/` contain a missed start or stop phrase. Proposed: when they do not, ask the user for a new recording session of the phrases in the situations where they fail (right after a previous dictation, in noise, with a short pause) before tuning anything.
- Target version 0.46.1 is proposed as a PATCH bump; the chat names no version.

## Release

SemVer: PATCH — a bug fix in an existing capability, no new command or behavior.
Target version: 0.46.1.
