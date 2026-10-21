## Why

The smoke scenario "A hands-free session in a synthesized voice" (`e2e-test/tests/features/voice_control.feature`, Rule "A whole hands-free session runs as said", steps in `e2e-test/tests/steps/voice_control/session.rs`) fails often: the stage journal holds 16 failures since 2026-10-19 on more than ten branches (`fix/bring-dictation-cursor-mic-closer`, `feat/dictation-archive`, `feat/add-voice-command-paste`, `fix/fix-reconnect-test-selection-race`, `feat/redesign-overlay-liquid-glass`, `feat/refinement-off-by-default`, `feat/settings-window`, `feat/restartable-app-runtime`, `fix/speed-up-short-start-cue`, `feat/permissions-gate-window`, `fix/match-late-confirmation-to-its-state`). It is the most frequent reason a `just check` or `just full-check` fails and has to be run again, each time at the cost of a lock wait of up to an hour.

The last failure (2026-10-21 07:05) expected `RecordingStarted, Cancelled, RecordingStarted` for the "cancel dictation" step and got `RecordingStarted, RecordingStarted`: the cancel was not carried out, and the next start was taken as a restart of the dictation still running. In the same run all three copies of the speech recognition service for dictation missed their 5.75 s deadline for one file, so the services were slow at that moment.

## What Changes

- The synthesized hands-free session gets every command of the session carried out, in order, in every run.

Current stage: PoC. Target stage: PoC.
Scope: why a command of the synthesized session (here "cancel dictation") is sometimes not carried out — the synthesized voice, the 30 s reaction wait of the steps, a slow service, or a real defect of voice control — and fixing the cause.
Acceptance bar: the scenario passes 10 `just e2e-smoke` runs in a row; its expected sequence of app actions stays as it is.
Out of scope: the other hands-free session scenarios, the dictation path.

## Capabilities

### Modified Capabilities

- `voice-control`: every command of a hands-free session in a synthesized voice is carried out.

## Impact

- `e2e-test/tests/steps/voice_control/session.rs`, `e2e-test/tests/features/voice_control.feature`
- possibly `command-spotting/` or `app/src/core/application/pipeline/voice_control.rs`, if the cause is a defect

## Open points

- Whether the failures share one cause (always the cancel step?) is not known; proposed: read the failed runs in the journal and logs first, in design.
- Target version: proposed the PATCH above `main` at the time it runs.

## Release

SemVer: PATCH — bug fix in voice control or its test.
Target version: 0.29.2.
