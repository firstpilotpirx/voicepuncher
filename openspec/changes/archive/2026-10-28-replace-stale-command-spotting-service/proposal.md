## Why

On 2026-10-27, after `reduce-voice-start-latency` raised the command spotting protocol to version 3, voice commands stopped working entirely: "start start" got no reaction. The app (with `[services] manage` on) adopted two copies of `voicepuncher-command-spotting` that an earlier app run had started from the bundle on 2026-10-22, on ports 49083 and 49084. Those copies speak protocol version 2, so the service closed every voice control session at once with the policy close "the app speaks protocol version 3, the service version 2", hundreds of times a minute, and the log only said that the command spotting service stopped answering.

Quitting the app does not stop adopted services, and `just update-app` (rebuild and relaunch) does not replace them, so the stale copies survive every restart. The only recovery was finding and killing the processes by hand.

## What Changes

- When the app adopts a running command spotting service, or any copy of it, it first checks the protocol version that copy speaks; a copy with another protocol version is never adopted.
- Such a copy is stopped and replaced by a fresh copy started from the app's own engines, on the same port.
- The log names the replacement in one line: the port, the old and the new protocol version.
- If a protocol mismatch still shows up later (a policy close naming the versions), the app treats it as a stale service and replaces it the same way, instead of reconnecting in a loop.
- When voice control connects and a copy refuses it for its protocol version, it moves on to the next copy, so commands keep working while the refused copy is replaced.

Current stage: PoC. Target stage: PoC.
Scope: the adoption of command spotting service copies in hotkey mode with `[services] manage` on.
Acceptance bar: with a copy speaking an older protocol on a command spotting port, the app starts, replaces that copy with its own and voice commands work, without any manual step.
Out of scope: the speech recognition services (whisper-server copies), the services started by `just start-model-servers` when `manage` is off, and changes to the protocol itself.

## Capabilities

### Modified Capabilities

- `local-services`: adoption of a running command spotting service checks its protocol version and replaces a mismatched copy.

## Impact

- The app's service supervision (adoption of running copies) and the command spotting client's handling of the protocol-mismatch close.
- The command spotting service itself does not change: its existing hello refusal already names both protocol versions.

## Decisions

- The adoption requirements live in `local-services`.
- The app learns a copy's protocol version from the hello handshake the service already answers (a refusal names both versions); no version query is added, and `/health` stays as it is.
- A mismatched copy the app did not start is stopped, but only when the process on that port is the command spotting service; any other program is left alone.

## Release

SemVer: PATCH — bug fix, voice control broken after a protocol bump.
Target version: 0.41.3.
