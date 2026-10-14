## Context

See proposal.md § Why for the incident. These facts of the current code shape the approach:

- **Command spotting service** (`command-spotting/src/server.rs`):
  - It runs one thread per connection and does everything in order: `socket.read()`, then `session.accept(samples)`, then `socket.send(event)`.
  - `accept` splits a message into 100 ms frames. For each frame it may call the speech recognition service several times, all blocking (`engine.rs`):
    - the head check once per utterance;
    - a running window every 1 s;
    - one confirmation per closed segment;
    - one per unheld phrase.
  - `confirm.rs` waits up to 10 s per request (`REQUEST_TIMEOUT`).
  - tungstenite answers a ping only when the thread reads again. So a ping is answered after every event for the audio sent before it. The app's `events_for_sent_audio` uses that ordering as a barrier. Only tests and E2E call it (`commandspotting_client_it.rs`, `voice_samples_it.rs`, `e2e-test/src/voice_commands.rs`).
- **App command spotting client** (`websocket/client.rs`, `websocket/mod.rs`):
  - It pings every 2 s and fails after 10 s without any message (`SILENCE_LIMIT`).
  - Any error sends `PipelineInput::SpottingLost` once and ends the worker thread.
  - `lose_spotting` stops the recording and turns listening off. Nothing reconnects.
  - The microphone capture (`capture.rs`) and the 10 s ring of heard audio (`heard.rs`) belong to one `Listening`.
- **App speech recognition client** (`transcription/openai/mod.rs`):
  - It uses ureq with one overall timeout, 60 s by default.
  - It retries only dropped connections: 200, 400 and 800 ms (`RETRY_PAUSES`).
  - The live worker (`pipeline.rs`) transcribes chunks one after another, logs a failed chunk and merges the rest. `stop_recording` waits for the worker without a bound.
- **App text refinement client:** it uses a 30 s timeout with no retry. `refine_or_raw` falls back to the raw transcript.
- **Service scripts** (`justfile`):
  - `supervise` restarts a service whose process exits. Nothing restarts a service that hangs.
  - HAProxy sends each copy one request at a time, gives short requests priority and uses TCP health checks. It has no `abortonclose`.
- **Evidence from the incident logs:**
  - whisper-server aborts a request when its client disconnects (`client disconnected, aborted processing`), so giving up on a hung request frees the copy.
  - The hang hit a 2.5 s English confirmation of Russian speech, once in each of two sessions. Its cause is unknown.
- **Tests:**
  - Component tests drive the real `Pipeline` with in-process doubles (`app/tests/steps/voice_control/doubles.rs`). There is no fake clock.
  - Adapter integration tests stand up raw `TcpListener` stubs (`transcription_openai_it.rs`, `commandspotting_client_it.rs`).
  - `command-spotting/tests/engine_it.rs` needs the real sherpa models. There is no fake speech recognition server.
  - E2E `local_services.feature` starts its own services on free ports and kills them with SIGKILL.

## Goals / Non-Goals

**Goals:**

- A slow or hung request to the speech recognition service delays only the check it belongs to. It never makes the app think the command spotting service is gone.
- Losing the command spotting service is temporary. The app comes back on its own and keeps the dictation.
- Every wait on a local service has a bound that fits what it waits for, and every retry is bounded and logged.
- A service that hangs without exiting is restarted by the scripts that already restart one that exits.
- Each failure mode has a deterministic test at the lowest level that can show it, plus an E2E scenario with the real services.

**Non-Goals:**

- No async runtime (tokio) in either crate. The design stays on std threads and channels, as the code does today.
- No change to how commands are recognized: the matcher, placements, thresholds and the confirmation audio are untouched (`fix-short-pause-after-command` owns the clip).
- No on-screen notice (`notify-service-failures`).

## Decisions

### 1. The command spotting service splits each connection into I/O, processing and confirmation

```
 app  <==== WebSocket ====>  [ I/O thread ]  owns the socket, read timeout 10 ms
                                |      ^     answers Ping at once (liveness)
                       audio,   |      |     writes events in the order queued
                       select,  v      |
                       sync  [ processing thread ]  Session: KWS + VAD, real time
                                |      ^
                     jobs       v      |  results, applied between frames
                             [ confirmation worker ]  HTTP to speech recognition,
                                                       deadline per attempt
```

- **I/O thread:**
  - It owns the `WebSocket` with a 10 ms read timeout.
  - It forwards binary audio, phrase selections and sync requests to the processing thread over a channel.
  - It sends queued events.
  - tungstenite's automatic pong goes out on the next read or write, so a ping is answered within ~10 ms whatever the processing does.
- **Processing thread:**
  - It owns the `Session`: keyword spotter, VAD, orderer and recent audio.
  - It never blocks on the network. A check that needs the speech recognition service becomes a `Job` with its audio and context (`head`, `window`, `segment`, `unheld`), sent to the worker.
  - Results come back on a channel and are applied between frames, through the existing `place` / `confirm` logic, now split into "build the job" and "apply the heard text".
  - The `Session` state a result needs (`reported_until`, `last_window`, `last_segment_end`) is read and written only on this thread. This keeps today's logic and its unit tests intact.
- **Confirmation worker:** one thread per connection, running jobs one at a time.
  - **Segment and unheld jobs** queue (bounded at 8; above that the oldest window or head job is dropped, never a segment job).
  - **Window and head jobs are skipped, not queued,** while a job of the same running utterance is in flight. The next due window covers the newer audio anyway.
  - **Deadline per attempt: 3 s.** A 2.5 s clip takes 0.1–0.5 s today, and HAProxy puts short requests ahead of long ones.
  - **Segment jobs are retried once** on a timeout or a dropped connection. They are the only chance to recognize a command said alone. The other jobs are not retried.
  - **Every job logs its duration:** debug when under 1 s, info when slower, warn when given up.
- **Event order:**
  - A segment's `speech_end` and its phrase or command go out together once its job is resolved (answered or given up). Today's order (start, phrases, end) stays.
  - Window and head commands go out when their result arrives, placed by their audio times as today. The app already orders by those times.
  - The keyword-only path is kept unchanged when there is no speech recognition service. When a segment job is given up, the keyword-spotted phrase alone decides, as it does when no service is configured.
- **Model-free seam:** confirmations go through a small trait in the crate, `Hear`: samples to `Heard` with a deadline. The processing logic can then be tested with a scripted `Hear`, and the server with a scripted session, without sherpa models. `UtteranceTranscriber` is the real implementation.
- **Alternatives considered:**
  - *Only lower `REQUEST_TIMEOUT` to 3 s.* It fixes the incident but not the class. Several calls per frame still add up, the stream still stalls for seconds, and the next slow-service mode returns the bug. Rejected as the whole fix, kept as the deadline.
  - *A pool of confirmation threads per connection.* Parallel requests to one service with three copies would compete with the dictation chunks and change the order results arrive in. Rejected; one worker with skip-while-busy is enough, because a check of a running utterance that is late is worthless.
  - *An async runtime.* It would be a large rewrite of a std-thread codebase for three threads per connection. Rejected.

### 2. "Every event for the sent audio" becomes an explicit sync message

- The app sends a text message `{"sync": n}`.
- The processing thread answers `{"event": "synced", "id": n}` once every frame received before it is processed and every job for those frames is resolved.
- Pings no longer carry this meaning; they only show that the connection lives.
- `events_for_sent_audio` sends the sync message and waits for `synced` with that id. It fails only when the connection goes silent: no pong for 5 s.
- Alternative: keep the ping ordering by having the I/O thread hold pongs. Rejected: it mixes liveness with processing again, which is the bug.
- Compatibility: an old app with the new service works, since pings are still answered. A new app's sync with an old service is never answered. That only affects tests, because `just run` and the E2E recipes start the service from the same checkout.

### 3. The app treats the service as gone only when the connection is dead, and then reconnects

- **Liveness:**
  - The client pings every 1 s and counts only pongs and messages as answers.
  - Five seconds without any answer, or a socket error or close, means the service is gone. A service that answers pings but spots slowly is fine (spec: "A slow command spotting service does not stop listening").
- **Reconnect** lives in the adapter worker (`websocket/mod.rs`); the core only learns about loss and return.
  - On loss the worker sends `PipelineInput::SpottingLost` once. It then tries `connect` with pauses of 0.25, 0.5, 1, 2 and then 4 s, forever, until told to stop, logging each failed attempt.
  - The 4 s cap keeps "back within 5 s" true.
  - The microphone capture and the heard-audio ring stay open across the outage. They move out of `AudioStream` into the worker's lifetime.
- **On a new connection:**
  - The worker sends the latest phrase selection it holds, from the control channel.
  - It replays the heard audio since `max(lost_at − 5 s, last_command_end)`, at most the ring's 10 s, then streams live audio.
  - It sets the client's `audio_start` to the instant of the first replayed sample, so event times map to the right instants.
  - It then sends `PipelineInput::SpottingBack`.
- **Core (`voice_control.rs`):**
  - On `SpottingLost`: log the error, play the listening-off sound and mark the spotter away. Listening stays on and the recording is not stopped.
  - On `SpottingBack`: reset the recognizer (`VoiceCommandRecognizer::new()`), point the service at the commands of the current state (`follow_dictation`), play the listening-on sound and log "voice control: listening again".
  - A command that ends at or before the end of the last command carried out is ignored, so replayed audio cannot carry a command out twice.
- **Alternatives considered:**
  - *Reconnect from the core, through the port.* It would put retry timing in the application layer and need a clock there. Rejected; the core reacts to two inputs, the adapter owns the network.
  - *Stop the dictation on loss, as today.* The user loses control of where the text goes and gets a paste mid-sentence into whatever has focus. Rejected (spec: "A dictation keeps recording while the command spotting service is away").
  - *No replay.* A "stop dictation" said during a restart would be lost. The replay is cheap because the ring already exists.

### 4. Speech recognition requests get a deadline that follows the audio, and timeouts and error answers are retried

- **Deadline per attempt:** `min(timeout_secs, 5 s + audio_secs / 2)`, about 21 s for a 32 s chunk and 7.5 s for a 5 s recording. A chunk takes about 0.2–2 s today, so this leaves wide room while bounding a hang.
- **Retries:**
  - **Timeouts and error answers** (HTTP 5xx and 429) are retried up to 2 more times, with no pause beyond the 200 ms first step. The service aborts the abandoned request, and HAProxy sends the next attempt to a free copy.
  - **Dropped connections** keep their own budget of 3 more attempts, unchanged.
  - **Other 4xx answers and unusable answers** are not retried.
- **Lost chunks:** a chunk that fails every attempt is logged with its number and audio length, and the other chunks are merged and pasted (today's live behavior, now specified and tested). The non-live path still drops the cycle when its single request fails every attempt.
- **Alternative:** a per-chunk retry in the pipeline. Rejected: the adapter already owns retries of dropped connections, and one place should own the attempt count.

### 5. Text refinement retries a dropped connection once

- The refinement client reuses the dropped-connection test of the transcription client. The function moves to a shared adapter module, `app/src/adapters/secondary/http_retry.rs`, used by both.
- It retries once after 200 ms. A timeout is not retried, because the raw transcript is already a good answer and a second 30 s wait is not.

### 6. The service scripts restart a service that stops answering

- **Health check in `supervise`:** next to `wait`, a background loop runs a service's health check every 5 s.
  - After 3 failures in a row it kills the service with SIGKILL.
  - The loop then logs `… stopped answering its health check — restarting` and restarts it with the usual back-off.
- **Command spotting:** `curl -fsS --max-time 2 http://127.0.0.1:<port>/health`.
  - The service answers this plain HTTP request on its WebSocket port before the handshake. The accept thread peeks at the request line.
  - It answers 200 while every connection's processing thread has made progress in the last 10 s whenever audio waits for it, and 503 otherwise.
  - A deadlocked processing thread or a hung accept loop therefore fails the check, while a slow confirmation does not.
- **Text refinement:** `curl -fsS --max-time 2 http://127.0.0.1:<port>/health`, as `llama_ready` already does.
- **Speech recognition copies:** no new check. A copy hung on one request is freed when its client gives up (decisions 1 and 4). HAProxy's TCP check already handles a copy that exits.
- **HAProxy:** add `option abortonclose`, so a queued request whose client gave up is dropped instead of sent to a copy.
- **Alternatives considered:**
  - *A WebSocket ping probe from bash.* It needs an extra binary or `websocat`. Rejected; HTTP on the same port is a few lines in `server.rs`.
  - *An HTTP health check of whisper-server copies.* A copy busy with a 30 s chunk would fail it and be marked down by mistake. Rejected.

### 7. Logs

- **Command spotting service:** every confirmation's duration; jobs given up and skipped; `processing lag` when the processing thread is more than 1 s behind the audio it has received.
- **App:**
  - each reconnect attempt, logged at info for the first and then every 10th;
  - `voice control: listening again`;
  - each retried speech recognition attempt, with its reason and duration.

## Test levels

| Spec scenario | Level | Where |
|---|---|---|
| voice-control / The app refuses to start without the command spotting service | E2E `.feature` (exists) | `e2e-test/tests/features/voice_control.feature` |
| voice-control / The app keeps reaching for the command spotting service when it goes away | component `.feature` + adapter integration | `app/tests/features/component/voice_control.feature` (replaces "Listening turns off when the command spotting service goes away"); `commandspotting_websocket_it.rs`: a stub that closes the connection, then reconnect attempts are logged |
| voice-control / Listening is on at start | component `.feature` (exists) | `voice_control.feature` |
| voice-control / Listening stays on after a hotkey dictation | component `.feature` (exists) | `voice_control.feature` |
| voice-control / Commands work again after the command spotting service restarts | E2E `.feature` + component `.feature` | E2E kills the real service on its own port and lets the scripts restart it; component sends `SpottingLost` then `SpottingBack` |
| voice-control / Listening is not given up after many failed attempts | adapter integration | `commandspotting_websocket_it.rs`: nothing listens for 20 s, then a stub starts on the port and `SpottingBack` arrives within 5 s |
| voice-control / A confirmation that takes longer than 10 seconds keeps listening on | E2E `.feature` (bug reproduction) | `voice_control.feature`, `@serial`: own speech recognition service with one copy, paused with SIGSTOP for 15 s during a hands-free dictation, then SIGCONT and "stop dictation" |
| voice-control / The hotkey ends a dictation while the service is away | component `.feature` | `voice_control.feature` |
| voice-control / "stop dictation" ends a dictation after the service is back | component `.feature` + E2E `.feature` | component: lost, back, then the spotted command; E2E: kill and restart the real service during a hands-free dictation |
| voice-control / "stop dictation" said during a short outage ends the dictation | adapter integration | `commandspotting_websocket_it.rs`: a stub drops the connection; the audio said during the outage reaches the next stub connection, checked through the replayed samples and the event times |
| speech-recognition / Service stops between recordings | E2E `.feature` (exists) | `speech_recognition.feature` |
| speech-recognition / Service is briefly busy, Drop reported as an invalid argument, Service keeps dropping the connection | adapter integration (exist) | `transcription_openai_it.rs` |
| speech-recognition / Service answers with an error | adapter integration (changed: 3 attempts) | `transcription_openai_it.rs` |
| speech-recognition / The first request of a chunk hangs | adapter integration + component `.feature` | `transcription_openai_it.rs`: a stub that never answers the first request; `speech_recognition.feature` (component): a transcriber double that fails the first attempt of chunk 2 |
| speech-recognition / One error answer is retried | adapter integration | `transcription_openai_it.rs` |
| speech-recognition / A hung short chunk is given up quickly | adapter integration | `transcription_openai_it.rs`: a stub that never answers, a 5 s recording, and the stop-to-failure time under 30 s |
| speech-recognition / The middle chunk of a long dictation fails | component `.feature` | `app/tests/features/component/speech_recognition.feature` |
| text-refinement / The text refinement service drops one connection | adapter integration | `textrefinement_openai_it.rs` |
| text-refinement / The text refinement service keeps dropping the connection | adapter integration | `textrefinement_openai_it.rs` |
| local-services / The command spotting service hangs | E2E `.feature` | `local_services.feature`, `@serial`: SIGSTOP the service started on own ports; it answers again within 30 s and its log names the health check |
| local-services / A busy service is not restarted | E2E `.feature` | `local_services.feature`: stream synthesized speech for 60 s to the service started with a speech recognition stub that answers in 4 s; no restart note in the log |
| local-services / One confirmation hangs | integration (crate) | `command-spotting/tests/server_it.rs`: the real server with a scripted session and a stub that never answers one request; pongs keep coming for 15 s and the give-up is logged |
| local-services / Commands are spotted after a hung confirmation | integration (crate, models) | `command-spotting/tests/engine_it.rs`: real models plus a stub speech recognition server that hangs the first request; "press enter" is reported |
| local-services / Checks of a long utterance are skipped while the service is slow | unit | the confirmation worker with a scripted `Hear` that takes 4 s; at most one window job is in flight |
| local-services / A command said alone is confirmed by the second attempt | unit + integration (crate, models) | unit: the worker retries a segment job; `engine_it.rs`: a stub that times out the first request |
| local-services / An abandoned request is not transcribed | E2E `.feature` | `local_services.feature`: one copy, a long request, then a second request whose client hangs up; the copy's log shows one request |

Beyond the scenarios, these unit tests cover the building blocks:

- the reconnect pause sequence and its cap;
- the replay start (`max(lost_at − 5 s, last_command_end)`, bounded by the ring);
- the deadline formula;
- the retry classifier (timeout, 5xx, 429, other 4xx, unusable answer, dropped connection);
- the duplicate-command guard in voice control;
- skip-while-busy and the bounded queue of the confirmation worker;
- the event order of a segment whose job resolves late;
- the `/health` answer: busy versus stuck;
- the sync request and its answer.

Fault injection:

- Adapter integration uses in-process `TcpListener` stubs that hang, close, reset or restart on the same port. They join the existing stubs in `transcription_openai_it.rs` and `commandspotting_client_it.rs`.
- E2E pauses (SIGSTOP/SIGCONT) or kills the real services started on free ports of their own (`steps/local_services.rs`), so the shared services on 8080–8082 are never touched. These scenarios are `@serial`.
- The new E2E scenarios get `@smoke` when they take under 60 s, and `@resilience` otherwise. `just e2e-resilience` runs that tag, and `full-check` gains it.

## Risks / Trade-offs

- **The event order changes inside the service.** A segment's end now waits for its confirmation, up to 6 s with the retry. The app already waits for confirmations in effect, since today they block the whole stream. → The orderer unit tests and the existing E2E suites (`@smoke`, `@accuracy`, `@voice_samples`) gate the change; any change in the WER or command accuracy numbers blocks the merge.
- **Replay can repeat a command** the old connection had already reported. → The core ignores a command that ends at or before the last one carried out (unit test). The replay starts no earlier than the end of that command.
- **A dictation started by voice could run unbounded** while the service is away for good. → The hotkey ends it, and the listening-off sound tells the user voice control is gone. On-screen notices come with `notify-service-failures`.
- **Retries add load** to a speech recognition service that is slow for everyone. → Attempts are bounded (3 per chunk, 2 per segment confirmation, none for checks of a running utterance). Abandoned requests are aborted by whisper-server and dropped from HAProxy's queue.
- **The health check could restart a service that is merely slow** (a false positive). → Health measures only whether processing makes progress, not how fast confirmations are. Three failures 5 s apart are needed, and "A busy service is not restarted" is an E2E scenario.
- **SIGSTOP-based E2E scenarios could leave a paused process** after a failed step. → The step guard sends SIGCONT and stops the services it started on drop, as `local_services.rs` already does for its services.
- **The deadline formula could cut off a chunk that is legitimately slow** under heavy load from three parallel copies. → The configured `timeout_secs` stays the upper bound. A cut-off attempt is retried on a free copy, and the slow-request log shows the margin.

## Migration Plan

- The app and the command spotting service are built and started from the same checkout. `just run` restarts the services before the app.
- Rollback is reverting the merge. No data or config migrates; `timeout_secs` keeps its meaning as the upper bound.
