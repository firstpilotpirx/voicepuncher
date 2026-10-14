# Proposal

## Why

On 2026-10-14, voice control died twice in the middle of a dictation and did not come back until the app was restarted. Each time the cause was one 2.5 s confirmation request that the speech recognition service took more than 10 s to answer (`failed to decode / client disconnected` in the whisper-server log). The command spotting service handles a connection on one thread and does everything in order: it read audio, waited for speech recognition and answered pings. So it went silent for exactly as long as its request timeout (10 s). That equals the app's silence limit, so the app decided the service was gone. It then stopped the dictation, turned listening off and never tried again, while the service itself was still alive.

The same weak points exist elsewhere:

- a dictation chunk whose request times out is lost, because timeouts are never retried;
- a single hung chunk holds the stop of the dictation for up to 60 s;
- the service scripts restart a service that exits but not one that hangs.

A PoC that is used every day must not lose voice control or dictated text because one request was slow or one connection dropped.

## What Changes

- **Command spotting service: processing that never stalls the connection.**
  - Each connection reads audio and answers pings on its own, independent of speech recognition.
  - Speech recognition confirmations run off the audio path, on a worker, each with a short deadline.
  - A slow, failed or hung confirmation delays or loses only that one check. It never ends the connection, and audio keeps being spotted meanwhile.
  - While a check of a running utterance is in flight, the next one is skipped, not queued, so a slow service cannot build up a backlog.
  - The confirmation that decides a whole utterance is a command is retried once.
  - Every confirmation logs how long it took.
- **App: voice control that comes back.**
  - The app treats the command spotting service as gone only when the connection drops or pings go unanswered. Waiting for spotting results no longer counts.
  - When the service is gone, the app reconnects in the background with a growing pause, for as long as voice control is on. It then resumes listening for the commands of its current state, playing the listening-off sound on loss and the listening-on sound on return.
  - Speech captured while the service was away is sent once it is back, so a command said during the outage still counts.
  - A dictation that runs while the service is away keeps recording. The hotkey still ends it, and voice commands end it again once the service is back.
- **Speech recognition: a slow or failed chunk is retried, not lost.**
  - A request that times out or gets an error answer is sent again, up to twice more. The service aborts the request the app gave up on, and the next attempt goes to a free copy.
  - The time the app waits for one request is scaled to the length of its audio instead of a flat 60 s, so a hung chunk holds the dictation for seconds, not minutes.
  - Only when every attempt fails is the chunk lost. The rest of the dictation is still pasted, as today, and the lost chunk is logged.
- **Text refinement:** a dropped connection is retried once before falling back to the raw transcript.
- **Service scripts: a hung service is restarted too.**
  - The scripts check that each service still answers, not only that its process runs: command spotting by a WebSocket ping, text refinement by its health endpoint. A service that fails several checks in a row is restarted and the restart is logged.
  - The load balancer of the speech recognition copies drops queued requests whose client gave up, so abandoned requests do not occupy copies.
- **Tests at every level**, unit, adapter integration, component and E2E. They reproduce the incident and cover a hung speech recognition copy, a hung or restarted command spotting service, a dropped connection and a slow chunk.
  - Fault injection uses in-process stub servers at adapter level.
  - At E2E level it pauses (SIGSTOP) or kills the real services.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `voice-control`: listening survives a stalled or restarted command spotting service, and a dictation keeps recording through it. This replaces "listening turns off when the command spotting service goes away".
- `speech-recognition`: a timed-out or failed request is retried, and the wait per request scales with the audio. This replaces "a timeout SHALL NOT be retried".
- `text-refinement`: a dropped connection is retried once before the raw-transcript fallback.
- `local-services`: a service that stops answering is restarted, a slow confirmation does not stall command spotting, and abandoned requests do not occupy speech recognition copies.

## Scope

- Current stage: PoC. Target stage: PoC, hardened for daily use against service stalls and losses.
- In scope: the command spotting service's connection handling and confirmations; the app's command spotting client and voice control on loss; the app's speech recognition and text refinement clients (deadlines, retries); the service scripts' health checks and load balancer settings; tests at every level; the glossary terms for the new behavior.
- Acceptance bar:
  - With one speech recognition copy paused for 30 s while the user dictates hands-free, listening stays on, and "stop dictation" ends and pastes the dictation.
  - With the command spotting service killed during a dictation, the dictation keeps recording. Within 5 s after the service is back, "stop dictation" ends it and pastes its text.
  - With the command spotting service paused for 30 s, the scripts restart it, and voice commands work again without restarting the app.
  - A dictation chunk whose first request hangs is transcribed by a retry, and its text is pasted.
  - Every scenario above is an automated test that fails on today's code: an E2E `.feature`, backed by component, adapter integration and unit tests.

## Out of scope

- On-screen notices of failures (`notify-service-failures`); this change uses only the existing sounds and logs.
- Starting, restarting or supervising services from the app; the service scripts stay the supervisor.
- Finding or patching why whisper-server sometimes decodes a short clip for more than 10 s.
- Keeping a dictation's audio to transcribe it again after the app gives up on a chunk.
- launchd agents or restarts across logins.
- Hang detection of a single speech recognition copy beyond request deadlines. A copy that hangs on one request is freed when the app or the command spotting service gives up on it.

## Release

MINOR, 0.12.0 → 0.13.0. The change goes beyond restoring the specified behavior: reconnecting voice control, retries of timeouts, health-based restarts, and a new message between the app and the command spotting service.

## Impact

- `command-spotting/`:
  - `server.rs` gets an I/O thread per connection, separate from processing;
  - `engine.rs` and `confirm.rs` move confirmations to a worker with deadlines, a single retry and skip-while-busy;
  - `protocol.rs` gets an explicit sync message for "every event for the audio sent so far", replacing the ordering of pings.
- `app/src/adapters/secondary/commandspotting/websocket/`: liveness by pongs, reconnect with backoff, replay of the audio captured during an outage, and the sync message.
- `app/src/core/application/pipeline/voice_control.rs`: lost and back are separate inputs; the dictation is kept; the sounds play on loss and on return.
- `app/src/adapters/secondary/transcription/openai/`: deadline per request scaled to the audio, and retries of timeouts and error answers.
- `app/src/adapters/secondary/textrefinement/openai/`: one retry of a dropped connection.
- `justfile`: health checks with restart in the supervisor, and `option abortonclose` in the HAProxy config.
- Tests:
  - new stub servers that hang, drop or restart in `app/tests/*_it.rs`;
  - a stub speech recognition server and a model-free engine seam in `command-spotting/tests/`;
  - component scenarios in `voice_control.feature`;
  - E2E scenarios in `voice_control.feature`, `speech_recognition.feature` and `local_services.feature` that pause or kill real services.
- `docs/ubiquitous-language.md`: terms for a service that is back, reconnecting, a confirmation deadline and a health check.
- Manual run by the user: recording and voice control behavior changes.
