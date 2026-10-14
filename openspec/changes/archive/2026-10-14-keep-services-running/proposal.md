## Why

Voice control silently stopped accepting any command: the speech recognition service (`whisper-server`) crashed (segmentation fault) on the first short utterance without speech, and nothing started it again, so every later command confirmation failed with "connection refused" while the app showed nothing. `just run` restarts the services only when it starts; a service that dies while the app runs stays dead.

Root cause of the crash: the command spotting service asks for the detailed answer format (`verbose_json`); the installed whisper.cpp server then also computes language probabilities; when voice activity detection leaves no speech, language detection returns an error id, the server turns it into a null language name and crashes. Reproduced: 2.3 s of silence in that format kills the server (exit 139); with language probabilities off (`-nlp`) the server answers with empty text and keeps running.

## What Changes

- The speech recognition service starts with language probabilities off, so audio without speech no longer crashes it. Nothing in voicepuncher reads the language probabilities.
- Every service that `start-model-servers` (and therefore `just run` and the E2E recipes) launches runs under a supervisor: when the service process exits while it is meant to run, it is started again after a short back-off, and the restart is written to the service's log.
- A service that exits before it ever became ready is reported as a start failure with its last log lines, as today, and is not restarted forever: no process is left behind.
- `stop-model-servers` stops the supervisor together with its service, so a stopped service stays stopped.
- `start-model-servers` takes the names of the services to start, like `stop-model-servers` already does (all three when none is named).
- A service's log is no longer truncated on a restart, so the output of the crash that caused it stays readable.

## Capabilities

### New Capabilities

- `local-services`: how the local services the app talks to (speech recognition, text refinement, command spotting) are started, kept running and stopped.

### Modified Capabilities

- `speech-recognition`: adds the requirement that audio without speech never stops the speech recognition service.

## Impact

- `justfile`: `start-model-servers` (supervisor, `-nlp`, service selection, log kept across restarts), `stop-model-servers` (stops the supervisor and its service).
- `e2e-test/`: new `local_services.feature` with steps; a new scenario in `speech_recognition.feature`; a dev-dependency on the `voicepuncher-command-spotting` crate to send the command confirmation request.
- `README.md`, `AGENTS.md`: services are restarted when they stop.
- No change to the app or the command spotting service code.

## Stage, scope and acceptance bar

- Current stage: PoC. Target stage: PoC (no stage change).
- Scope: the crash on audio without speech, and keeping the three local services running while they are meant to run.
- Acceptance bar: the new E2E scenarios pass in `just check` and `just full-check`; a silent utterance sent in the command confirmation format leaves the speech recognition service answering; a killed service answers again within 30 s; a stopped service stays stopped; a service that cannot start is reported and leaves no process.

## Out of scope

- Telling the user in the app's own log or overlay that the speech recognition service is unreachable while confirming commands (the command spotting service logs it; the app does not learn of it). A separate change.
- Restarting services after the machine reboots or the user logs out (launchd agents).
- Fixing the crash in whisper.cpp itself.

## Version

MINOR, target version 0.11.0: a bug fix that also adds the `local-services` capability, and a new capability is MINOR before 1.0.
