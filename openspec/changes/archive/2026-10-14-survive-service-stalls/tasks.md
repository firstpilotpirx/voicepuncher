# Tasks

## 1. Reproduce the incident

- [x] 1.1 Red (bug reproduction): add the scenario "A confirmation that takes longer than 10 seconds keeps listening on" to `e2e-test/tests/features/voice_control.feature` (`@serial`), with steps that start an own speech recognition service (one copy) and command spotting service on free ports, run a hands-free dictation, pause the speech recognition service with SIGSTOP for 15 s, resume it and say "stop dictation"; the step guard sends SIGCONT and stops the services on drop; run it and see it fail with "command spotting service stopped answering"; commit the intentionally red test on its own
- [x] 1.2 Red (bug reproduction, crate level): add `command-spotting/tests/server_it.rs` "One confirmation hangs" against the real server with a stub speech recognition server that never answers one request; run it and see pongs stop for 10 s; commit the intentionally red test on its own

## 2. Command spotting service: a model-free seam for confirmations

- [x] 2.1 Red: in `engine_it.rs` (real models), a session whose confirmations come from a scripted `Hear` that hears "press enter" reports it for an utterance the keyword spotter alone would not take; run it and see it fail to compile, as the seam does not exist
- [x] 2.2 Green: introduce the `Hear` trait (samples → `Heard` within a deadline) with `UtteranceTranscriber` as its implementation; split `place`/`confirm` into "build the job" and "apply the heard text"; 2.1 passes and every existing unit test and `engine_it.rs` stays green (`just test`)
- [x] 2.3 `just check` and commit

## 3. Command spotting service: confirmations off the audio path

- [x] 3.1 Red: unit tests for the confirmation worker and the outbox — the outbox is busy while a check waits (skip-while-busy, proven end to end by `engine_it.rs` with a `Hear` taking 4 s) while segment jobs always queue; a segment job not answered in time is tried once more; a head job is not retried; run them and see them fail
- [x] 3.2 Green: the confirmation worker thread with a 3 s deadline per attempt (`--confirmation-deadline-secs`), skip-while-busy and one retry of segment jobs; each job logs its duration (debug < 1 s, info slower, warn when given up)
- [x] 3.3 Red: unit test for the event order — a segment whose job resolves late still emits speech start, the phrases spotted in it and speech end, then its confirmed phrase or command, as today; a given-up segment job leaves the keyword-spotted phrase to decide; run it and see it fail
- [x] 3.4 Green: the processing thread holds a segment's end events until its job is resolved and applies results between frames
- [x] 3.5 Red: add "Commands are spotted after a hung confirmation" and "A command said alone is confirmed by the second attempt" to `command-spotting/tests/engine_it.rs` (real models, stub speech recognition server that hangs or times out the first request); run them and see them fail
- [x] 3.6 Green: wire the worker into `Session`; the scenarios of 3.5 pass
- [x] 3.7 `just check` and commit

## 4. Command spotting service: I/O independent of processing, sync message

- [x] 4.1 Red: unit tests for the sync message — `{"sync": n}` is answered with `{"event":"synced","id":n}` only after every earlier frame is processed and every job for it resolved; run them and see them fail
- [x] 4.2 Green: the sync message in `protocol.rs` and its handling on the processing thread
- [x] 4.3 Green: the I/O thread per connection (socket with a 10 ms read timeout, channels to and from the processing thread, pongs answered at once); the reproduction test 1.2 passes
- [x] 4.4 Red: unit tests for `/health` — 200 while every processing thread with waiting audio made progress in the last 10 s, 503 when one did not, 200 with no connection; run them and see them fail
- [x] 4.5 Green: the accept thread answers `GET /health` on the WebSocket port before the handshake; the `processing lag` log when the processing thread falls more than 1 s behind
- [x] 4.6 `just check` and commit

## 5. App: liveness by pongs and the sync message

- [x] 5.1 Red: in `commandspotting_client_it.rs`, a stub that answers pings but sends its events 12 s late keeps the client alive; a stub that stops answering pings is reported gone after 5 s (within 7 s); `events_for_sent_audio` waits for `synced` with its id; run them and see them fail
- [x] 5.2 Green: ping every 1 s, a 5 s silence limit counting any answer, `events_for_sent_audio` on the sync message; update `voice_samples_it.rs` and `e2e-test/src/voice_commands.rs` callers if their signatures change; `just test`
- [x] 5.3 `just check` and commit

## 6. App: reconnect, replay and voice control that comes back

- [x] 6.1 Red: unit tests in the adapter for the reconnect pauses (0.25, 0.5, 1, 2, then 4 s), the replay start `max(lost_at − 5 s, last_command_end)` bounded by the 10 s ring, and the `audio_start` of the new connection; run them and see them fail
- [x] 6.2 Green: the worker keeps the microphone capture and the heard-audio ring across an outage, reconnects with those pauses, sends the latest phrase selection, replays the audio and sends `SpottingLost` once and `SpottingBack` on return; failed reconnect attempts are logged (the first, then every 10th, naming the session and the service address)
- [x] 6.3 Red: in `commandspotting_reconnect_it.rs` — "The app keeps reaching for the command spotting service when it goes away" (stub closes, attempts logged), "Listening is not given up after many failed attempts" (nothing listens for a minute, then a stub starts on the port and `SpottingBack` arrives within 5 s; the first and the tenth failed attempt are logged), and "\"stop dictation\" said during a short outage ends the dictation" (the stub refuses connections for 3 s; the audio heard meanwhile reaches the next connection and its events are placed at the capture time of the first sample sent again); run them and see them fail
- [x] 6.4 Green: the adapter passes 6.3; replace `should_report_the_service_lost_when_it_stops_answering` with its reconnecting version
- [x] 6.5 Red: in `app/tests/features/component/voice_control.feature` replace "Listening turns off when the command spotting service goes away" with "The app keeps reaching for the command spotting service when it goes away", and add "Commands work again after the command spotting service restarts", "The hotkey ends a dictation while the service is away" and "\"stop dictation\" ends a dictation after the service is back", with steps that send `SpottingLost` and `SpottingBack`; run them and see them fail
- [x] 6.6 Red: unit test in `pipeline/voice_control.rs` — a command that ends at or before the last command carried out is ignored after `SpottingBack`; run it and see it fail
- [x] 6.7 Green: `PipelineInput::SpottingBack`; `lose_spotting` logs, plays the listening-off sound and keeps the recording; on back the recognizer resets, the current state's commands are selected, the listening-on sound plays and "voice control: listening again" is logged; the duplicate guard; 6.5 and 6.6 pass
- [x] 6.8 Add "Speech captured while the service is away", "Back (command spotting service)" and "Reconnect" terms to `docs/ubiquitous-language.md` and update the "Listening" and "Command spotting service" entries
- [x] 6.9 `just check` and commit

## 7. App: speech recognition deadlines and retries

- [x] 7.1 Red: unit tests for the deadline `min(timeout_secs, 5 s + audio_secs / 2)` and the retry classifier (timeout, 5xx and 429 retried up to 2 more times; other 4xx and unusable answers not; dropped connections keep their 3 more attempts); run them and see them fail
- [x] 7.2 Red: in `transcription_openai_it.rs` — "The first request of a chunk hangs" (stub never answers the first request), "One error answer is retried", "A hung short chunk is given up quickly" (stub never answers; a 5 s recording fails within 30 s, every attempt logged), and "Service answers with an error" now sent 3 times; run them and see them fail
- [x] 7.3 Green: the deadline per attempt and the retries, each failed attempt logged with its reason and duration; move the dropped-connection test to `app/src/adapters/secondary/http/failure/mod.rs`, with its own `http_failure_it.rs`
- [x] 7.4 Red: in `app/tests/features/component/speech_recognition.feature` add "The middle chunk of a long dictation fails" (the first-request hang is retried inside the adapter, so it is tested in 7.2 only) (chunk 2 fails every attempt; chunks 1 and 3 pasted, chunk 2 logged as lost); run them and see them fail where the behavior is missing
- [x] 7.5 Green: the live worker logs a lost chunk with its number and audio length and pastes the rest; 7.4 passes
- [x] 7.6 `just check` and commit

## 8. App: text refinement retries a dropped connection once

- [x] 8.1 Red: in `textrefinement_openai_it.rs` — "The text refinement service drops one connection" (refined text, dropped attempt logged) and "The text refinement service keeps dropping the connection" (two requests, raw transcript); run them and see them fail
- [x] 8.2 Green: one retry after 200 ms through `http/failure`; no retry of timeouts
- [x] 8.3 `just check` and commit

## 9. Service scripts: health checks and abandoned requests

- [x] 9.1 Red: add "The command spotting service hangs" to `e2e-test/tests/features/local_services.feature` (`@serial`: SIGSTOP the service started on own ports; it answers again within 30 s and its log names the health check); run it and see it stay hung
- [x] 9.2 Green: the health loop in `supervise` (every 5 s, `curl --max-time 2` on `/health`, SIGKILL and a restart note after 3 failures in a row) for the command spotting and text refinement services
- [x] 9.3 Red: add "A busy service is not restarted" (60 s of synthesized speech streamed to a command spotting service whose speech recognition stub answers in 4 s; no restart note) and "An abandoned request is not transcribed" (two copies busy with long speech, a third request whose client gives up; the copy that takes it aborts it) to `local_services.feature`; run them and see the second fail
- [x] 9.4 Green: `option abortonclose` in the HAProxy config, with a comment on why
- [x] 9.5 `just check` and commit

## 10. E2E: voice control through outages

- [x] 10.1 Red: add "Commands work again after the command spotting service restarts" (kill the real service on own ports, the scripts restart it, "press enter" presses Return 5 s after it answers) and "\"stop dictation\" ends a dictation after the service is back" (kill and restart during a hands-free dictation; the text is pasted once, after the service is back) to `e2e-test/tests/features/voice_control.feature` (`@serial`); run them and see them fail where the behavior is missing
- [x] 10.2 Green: fix what 10.1 shows; the reproduction scenario of 1.1 passes
- [x] 10.3 Tag the new E2E scenarios `@resilience @serial` (each starts services of its own); add `just e2e-resilience` (machine lock, own ports) and run it from `full-check`; update AGENTS.md § How to verify a change and the README test section
- [x] 10.4 `just check` and commit

## 11. Tracing

- [x] 11.1 Red: unit tests that the log lines of a confirmation job, a reconnect and a chunk attempt carry their ids (`conn`, `job`, `session`, `chunk`, `attempt`) through a captured logger (`job=` and the worker thread's name in `worker.rs`, `session=` in `commandspotting_reconnect_it.rs`, `request=` in `transcription_openai_it.rs`); run them and see them fail where an id is missing
- [x] 11.2 Green: thread names and ids on every line per design § 7 in both crates, the 30 s `stats` line of the service, the thread name in both log formats; document how to trace a session in README § Troubleshooting
- [x] 11.3 `just check` and commit

## 12. Version and manual checks

- [x] 12.1 Bump `version` in `app/Cargo.toml` to 0.13.0; `just check` and commit
- [x] 12.2 Manual checks by the user (recording and voice control changed): with `just run`, (a) dictate hands-free for a minute and say "stop dictation" — text pasted, no "stopped answering" in the log; (b) during a hands-free dictation run `kill -9` on the command spotting service — the listening-off sound plays, the dictation keeps recording, the listening-on sound plays when the scripts restarted it, and "stop dictation" pastes the text; (c) the hotkey still records and pastes with and without `--no-llm`; paste the log
