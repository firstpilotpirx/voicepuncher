## Context

The command spotting adapter (`app/src/adapters/secondary/commandspotting/websocket/mod.rs`) opens `MicInput` once in `start_listening`, hands its ring consumer to the streaming worker and keeps the `MicInput` alive in `Listening::_capture`. Nothing reads the capture's `CaptureHealth`, so a CoreAudio input that stops calling back without an error leaves voice control deaf until the app restarts (proposal § Why). The dictation recorder has its own watchdog (`recording/cpal/watchdog.rs`) because it opens a fresh stream per press.

Services the app starts write through pipes into copy threads of the app (`localservices/process/mod.rs`); when that app quits, the pipes close, and a service a later app adopts writes into nothing.

## Goals / Non-Goals

**Goals:** voice control recovers on its own from a silent stall or an input error of its microphone; an adopted service's log goes on.

**Non-Goals:** the root cause of the CoreAudio stall; the dictation stream; the service protocol; falling back to another input device.

## Decisions

### A microphone keeper thread owns the command spotting microphone

New module `commandspotting/websocket/kept_mic.rs`: `KeptMic::start(open, timings)` spawns a thread `command-spotting-mic` that calls `open` (a closure returning an `OpenMic`: its audio as `CapturedAudio`, its `CaptureHealth`, its rate, when it started and the device handle to keep alive) and owns what it returns, so the cpal stream never leaves the thread that opened it. Every tick (20 ms) the keeper takes the captured audio, resamples it to 16 kHz and sends it over a channel; the streaming worker reads that channel (`Receiver<Vec<f32>>` is already a `CapturedAudio`) and `AudioStream` is started at 16 kHz, so a reopened device with another rate changes nothing downstream. The first open runs before `start` returns, so a microphone that cannot open still fails `start_listening` as today.

Alternative rejected: reading the health from the streaming worker and reopening there — the cpal stream would have to cross threads, and the worker would mix device recovery with the service's outage handling.

### Stall rule and retries

A microphone counts as stalled when its health says `failed`, or when no callback came for `stall` (2 s) after it delivered audio, or for the dictation watchdog's `START_LIMIT` (5 s) before its first audio. The keeper logs `command spotting: microphone <reason>; opening it again` once, drops the device, and calls `open` again at once and then every `retry` (1 s) until it succeeds; the first failed reopen is logged once, and success logs `command spotting: microphone open again at <rate> Hz after <ms> ms`. The same input device name is opened (the closure captures it). `KeeperTimings { stall, retry, tick }` lets tests use milliseconds.

### The silent time is streamed as silence

The service and `HeardAudio` count time by samples from the first capture start. After a reopen the keeper sends zeros for the gap between the samples it sent and the new device's start time at 16 kHz, so later events keep their real capture times and a voice start's lead-in stays right.

### Services write their log file directly

`ProcessLauncher::spawn` opens the `RotatingFileWriter` as now, writes its start line, then takes its file (`into_file`) and gives the child two handles of it as stdout and stderr (`Stdio::from(file.try_clone())`). The copy threads, their join limit and their two unit tests go away; the integration test checks that the service's output is not a pipe. Rotation now happens only at a start, as the scripts do; a service's log can grow past the limit during one long run, which is accepted for a PoC.

Alternative rejected: having each service open its log itself — whisper-server is a third-party binary.

## Test mapping

| Scenario | Test |
|---|---|
| A microphone that stops delivering audio is opened again (voice-control) | unit `kept_mic.rs` `should_open_a_stalled_microphone_again_and_send_its_audio` (the stall and the reopen logged once each), `should_wait_two_seconds_and_retry_every_second_by_default`, `should_give_a_microphone_five_seconds_for_its_first_audio` |
| A microphone that reports an error is opened again (voice-control) | unit `kept_mic.rs` `should_open_a_microphone_that_reported_an_error_again` |
| A microphone that cannot be opened again is retried (voice-control) | unit `kept_mic.rs` `should_retry_a_failed_reopen_until_it_succeeds` (the failure logged once) |
| The silent time keeps the capture times (voice-control) | unit `kept_mic.rs` `should_send_the_silent_time_as_silence` |
| A started service writes its own log (local-services) | integration `app/tests/it/localservices_process.rs` `should_have_a_service_write_both_output_streams_into_its_log_file_itself`; unit `supervision_tests.rs` `should_start_a_service_nothing_answers_for_and_count_it_ready_once_it_answers` (the ready line) |
| A service keeps its log after the app that started it quits (local-services) | integration `app/tests/it/localservices_process.rs` `should_have_a_service_write_both_output_streams_into_its_log_file_itself` (its output is the file, not a pipe of the app) |

These are adapter behaviors (a cpal device, a child's file handles) with no domain decision, so they are tested at unit and adapter-integration level, not in a `.feature`.

## Risks / Trade-offs

- A device that keeps stalling is reopened every 2 s and logs each stall → acceptable; the log shows the pattern, and listening keeps working between stalls.
- Padding with silence after a long stall sends the whole gap as zeros at once → bounded by the stall and retry time; the service handles silence.

## Migration Plan

None: the change ships in the app; services started by an earlier version keep their old (closed) output until they restart.
