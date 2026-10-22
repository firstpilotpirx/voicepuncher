# Design

## Context

See proposal.md § Why. Both microphone users are in `app/`: the recorder (`recording/cpal/`) and the command spotting capture (`commandspotting/websocket/capture.rs`). Adapters reach the pipeline through `PipelineInput`. `recording/cpal/mod.rs`, `start`, `live_watcher_thread` and `pipeline.rs` are in the size baseline and may only shrink.

## Goals / Non-Goals

**Goals:** a wait-free input callback; one continuous 16 kHz recording buffer; a device failure as a pipeline input.

**Non-Goals:** the chunking algorithm; the `AudioRecorder` port; a device failure while voice control only listens (logged).

## Decisions

| Decision | Chosen | Why | Rejected |
|---|---|---|---|
| Ring buffer | `rtrb = "=0.4.0"`, `f32` mono, 2 s at the native rate, allocated at start | wait-free SPSC made for audio threads | `ringbuf`; a channel (allocates per send) |
| Shared capture | `recording/cpal/capture.rs`: `MicInput` opens the device; a free callback function averages frames to mono, pushes into the ring, stores level, overflow and callback time in atomics; the error callback sets a flag | one callback for both, tested under a counting allocator | a new crate |
| Shared resampler | `recording/cpal/resampler.rs`: `StreamResampler` keeps rubato's state; `push` feeds whole 1 024-frame blocks, `finish` processes the remainder and flushes the delay | no reset, no padding, one copy | two copies (today) |
| One 16 kHz buffer | a drain thread pops the ring every 5 ms, resamples, appends at 16 kHz; the live watcher, `truncate_at`, `begin_with` and `stop` (after `finish`) work on it | chunks are slices of one continuous stream: no filter restart, no padding, no lead-in upsampling | one filter fed chunk by chunk (overlap fed twice) |
| Overflow | the callback counts dropped samples; `stop` logs the lost ms once | never silent, never logged on the audio thread | dropping silently (today) |
| Device failure | the drain thread sends `PipelineInput::RecordingFailed(reason)` on the error flag or after 1 s without a callback once the device delivered audio, 5 s before its first frame (a vanished device CoreAudio may not report; a device takes up to a few seconds to start); `runtime/wiring.rs` hands the recorder the sender | no port change | polling the recorder |
| Pipeline reaction | `pipeline/recording_failure.rs`: while recording, stop as a user stop with the pending notice `PartOfDictationLost(Microphone)`, or `DictationLost(Microphone)` when nothing usable was recorded; otherwise ignored | reuses the lost-chunk notice path | a new notice variant |
| Splitting | `capture.rs`, `resampler.rs`, `drain.rs`, `watcher.rs` under `recording/cpal/`; unused `new_with_live_buffer` goes | the listed units shrink and leave the baseline | — |

## Risks / Trade-offs

- [Watchdog fires on a system sleep] → 1 s is ~100 callbacks; a false failure keeps the audio.
- [`cpal` reports no error for an unplugged device] → the watchdog is the primary signal; the manual run unplugs one.
- [The drain thread falls behind] → 2 s of ring; the loss is counted and logged.
- [`pipeline.rs` grows by a dispatch arm] → `stop_after_failure` moves out.

## Test levels

Adapter integration: `recording_cpal_it`, synthetic 48 kHz stream. Component: the capability's `.feature`, stand-in recorder.

| Scenario | Level |
|---|---|
| A long dictation on a busy machine is complete | adapter integration (60 s ramp beside CPU hogs) |
| Audio delivered while the recording is held up is kept | adapter integration |
| Audio dropped is reported once | adapter integration (log captured) |
| The final chunk ends where the dictation ends | unit (`resampler.rs`), adapter integration |
| The last words before the stop are whole | unit (`resampler.rs`), adapter integration |
| Chunks of one dictation join without a gap | unit (`resampler.rs`), adapter integration |
| The microphone disappears mid-dictation | component, adapter integration (watchdog), manual run |
| The microphone reports an error mid-dictation | component, adapter integration (error flag) |
| Nothing recorded before the failure | component |
| The microphone fails during a hotkey dictation | component |
| The microphone fails during a voice dictation | component |
| the callback allocates nothing (acceptance bar) | adapter integration, counting allocator |
