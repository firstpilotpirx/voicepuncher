## Why

Every part of voicepuncher should bring itself back after a crash, without a manual step. Today it does not. On 2026-10-27 the two copies of the command spotting service (`voicepuncher-command-spotting`, ports 49083 and 49084) stopped. The app logged "command spotting service: copy 1 of 2 stopped — it was started outside the app" and kept trying to reach the service, but never started a new copy. The menu bar icon showed the warning sign, and voice commands stayed dead until the user restarted the app by hand.

![Menu bar icon with the warning sign while the command spotting service was down](assets/0154ab556057.png)

The user wants every component of the system to restart itself after a crash as far as possible. That matters most for the command spotting service and for the speech recognition services, the whisper-server copies that serve dictation and commands.

## What Changes

- When a copy of a local service stops or stops answering, the app starts a new copy from its own engines on the same port. This covers every copy of the command spotting service, of the speech recognition service for dictation and of the speech recognition service for commands.
- A copy the app adopted is restarted the same way as one it started itself. A copy that was "started outside the app" no longer stays dead.
- Restarts back off when a copy keeps crashing. After repeated failures the app says so in the log and in the menu, naming the service and the reason, instead of retrying silently forever.
- The warning sign in the menu bar clears by itself once the restarted copy answers again.
- Every restart is logged in one line: the service, the copy, the port and the reason.

Current stage: PoC. Target stage: PoC.
Scope: crash recovery of the local services in hotkey mode with `[services] manage` on: the command spotting service and both speech recognition services, every copy.
Acceptance bar: kill any single copy of any of the three services while the app runs; within a few seconds a new copy answers on the same port, and dictation and voice commands work again without the user doing anything.
Out of scope: the services run by `just start-model-servers` with `manage` off (the script already restarts them), and replacing a copy that speaks an older protocol (the change `replace-stale-command-spotting-service`).

## Capabilities

### Modified Capabilities

- `local-services`: the app restarts every stopped copy of a local service, whether it started the copy or adopted it, with a back-off and a visible state when restarts keep failing.

## Impact

- The app's service supervision: adoption, watching and restarting copies.
- The menu bar state of the services: the warning sign and its clearing.

## Open points

- Which other components should restart themselves (for example the audio capture or the hotkey listener after a failure): proposed — only the three local services in this change; anything else to be named at the design stage.
- Back-off and give-up limits: proposed — restart at once, then back off up to about 30 s, and show the failure in the menu after 5 failed restarts in a row while still retrying.
- Whether an adopted copy may be replaced at all: proposed yes, since it holds the app's own port.
- Possible overlap with `replace-stale-command-spotting-service`, which touches the same adoption code: proposed — run that change first and declare the dependency at the design stage.

## Release

SemVer: PATCH — bug fix, services that crash stay down.
Target version: 0.41.2.
