## Why

The acceptance run of v0.36.1 (`docs/acceptance-runs/2026-10-22.md`, item 9.1) sent the installed app a quit from the system — the Apple Event quit that logout, shutdown and Activity Monitor's Quit send (`osascript -e 'tell application id "io.github.firstpilotpirx.voicepuncher" to quit'`). The app ended without any log line and left its three owned `whisper-server` copies (ports 18082–18084) running, orphaned with parent pid 1. The next launch adopted them (`services: adopted speech recognition service for dictation: copy 1 of 3`), so no later Quit ever stops them: they hold memory and the GPU until the user kills them by hand.

## What Changes

- Every quit the app accepts — menu Quit, a quit from the system (logout, shutdown, Activity Monitor), SIGTERM — is logged and stops every service the app started, as menu Quit does today.
- A regression test sends the system quit to a running app and checks its owned services are gone.

Current stage: PoC. Target stage: PoC.
Scope: the app's shutdown path for quits that do not come from its menu.
Acceptance bar: after a system quit or SIGTERM, the log names the quit and no service the app started is left running; the regression test passes.
Out of scope: services the app adopted (they stay running, as specified); the old `haproxy` proxies an older version's scripts left on ports 8081 and 8085.

## Capabilities

### Modified Capabilities

- `local-services`: "Quit stops every owned service" covers a quit from the system and SIGTERM, not only the menu.

## Impact

- The app's quit handling (application delegate / quit event, signal handling) and the shutdown of owned services.
- A new test that quits a running app from outside.

## Open points

- Whether SIGINT (Ctrl-C in a terminal run) takes the same path — proposed: yes.
- How long a system quit may wait for the services to stop before the app exits — proposed: the same limit menu Quit uses.

## Release

SemVer: PATCH — a bug fix.
Target version: 0.37.1 (batch; the orchestrator fixes the final number).
