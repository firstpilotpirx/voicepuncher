# Tasks

## 1. Shared stream resampler

- [x] 1.1 Red: unit tests in `app/src/adapters/secondary/recording/cpal/resampler.rs` — `finish` yields the remainder and the filter delay without trailing zeros, chunked pushes equal one whole pass, 16 kHz passes through; run `just test`, see them fail
- [x] 1.2 Green: `resampler.rs` with `StreamResampler::{new, push, finish}` built on rubato, declared in `cpal/mod.rs`
- [x] 1.3 `commandspotting/websocket/mod.rs` uses the shared resampler; its copy and the duplicated constants leave `capture.rs`; `just test`, commit

## 2. Wait-free shared capture

- [x] 2.1 Red: `app/tests/capture_allocations_it/main.rs` with a counting global allocator — the input callback on mono and stereo frames allocates nothing; unit tests in `capture.rs` — frames averaged to mono, the level stored, overflow counted when the ring is full, the error callback sets the flag; run them, see them fail
- [x] 2.2 Green: pin `rtrb = "=0.4.0"` in `app/Cargo.toml`; `capture.rs` with `MicInput::open` (device, ring sized for 2 s, health atomics) and the free callback function
- [x] 2.3 `commandspotting/websocket/mod.rs` pops the ring consumer instead of the mpsc of `Vec<f32>`; `MicCapture` goes; `just test`, commit

## 3. One 16 kHz recording buffer

- [x] 3.1 Red: `app/tests/recording_cpal_it/{main,synthetic}.rs` on a synthetic 48 kHz stream through the ring and drain — 60 s ramp beside CPU hogs keeps every sample in order, a drain paused for a second catches up with nothing missing, a ring overrun logs the lost ms once at stop, the recording ends with the stream's last samples and no trailing zeros, live chunks laid end to end equal the whole recording; run them, see them fail
- [x] 3.2 Green: `drain.rs` (ring → resampler → 16 kHz buffer every 5 ms, overflow logged once at stop) and `watcher.rs` (`live_watcher_thread` on the 16 kHz buffer, no resampling); `mod.rs` keeps the buffer at 16 kHz, `stop` calls `finish`, `NativeRecording`, `upsampled` and `new_with_live_buffer` go
- [x] 3.3 `just size-baseline` drops `start`, `live_watcher_thread` and shrinks `cpal/mod.rs` in `app/tests/size_baseline.toml`; `just test`, commit

## 4. A microphone failure ends the dictation

- [x] 4.1 Red: scenarios "The microphone disappears mid-dictation", "The microphone reports an error mid-dictation" and "Nothing recorded before the failure" in `app/tests/features/component/speech_recognition.feature`, "The microphone fails during a hotkey dictation" and "The microphone fails during a voice dictation" in `failure_notices.feature`, with steps whose stand-in recorder sends the failure input; `recording_cpal_it/synthetic.rs` — the error flag and a silent stream for 1 s each deliver `RecordingFailed`; run them, see them fail
- [x] 4.2 Green: `PipelineInput::RecordingFailed(String)`; `pipeline/recording_failure.rs` stops the recording with the pending notice (`PartOfDictationLost` or `DictationLost`, part Microphone), `stop_after_failure` moves there; the dispatch arm in `pipeline.rs` and `pipeline/voice_control.rs`; the drain thread sends the input; `runtime/wiring.rs` hands the recorder the sender
- [x] 4.3 Dispatch unit tests in `pipeline/recording_failure.rs` (stale input ignored, hold-mode release after the failure ignored); `just test`, commit; `just check` once

## 5. Version and manual checks

- [x] 5.1 Bump `version` in `app/Cargo.toml` to the next PATCH above `main` (`kit version-check realtime-safe-audio-capture`); `just test`, commit
- [x] 5.2 Manual checks by the user: a hotkey dictation and a voice dictation paste as before; a dictation with an external microphone unplugged mid-sentence pastes the words before the unplug and shows the microphone notice; the log shows no dropped-audio line in a quiet run

## Workflow follow-up

- Archive the change after verify passes.
