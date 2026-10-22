## Why

The CoreAudio input callback (`app/src/adapters/secondary/recording/cpal/mod.rs:139–170`) builds a `Vec<i16>` per buffer and takes `buffer_write.lock()` on the `std::sync::Mutex` the watcher thread holds while slicing and copying chunks (`:444, :462, :466`); `commandspotting/websocket/capture.rs:41–43` does the same per buffer. On a realtime thread an allocation or a contended lock is a dropout, which surfaces as missing audio, never as an error (audit of 2026-10-22). The resampler zero-pads the last chunk to 1 024 frames (trailing silence sent to speech recognition) and resets the sinc filter on every live chunk (`:377–409`), so every seam restarts the filter; the resampler parameters and `to_mono` are declared twice (`cpal/mod.rs:22–25`, `capture.rs:16–19`). The error callback only logs (`:171–173`); a device change is not handled; `#[allow(dead_code)] channels` at `:58`.

## What Changes

- The input callback allocates nothing and takes no lock: a single-producer single-consumer ring buffer (`rtrb` or `ringbuf`, pinned) carries samples to the watcher thread; the level register stays atomic; overflow is counted and logged once per recording, never silently dropped.
- One resampling module shared by the app and the command spotting capture: no reset between live chunks of one recording, the filter delay flushed at the end so the last words are whole, no zero-padding sent to the service.
- The stream error callback ends the recording with a failure notice; a device that disappears mid-recording is a failure notice, not silence.
- `live_watcher_thread` (108 lines) and `start` (88) split under the budgets.

Current stage: PoC. Target stage: PoC.
Scope: `recording/cpal/`, `commandspotting/websocket/capture.rs`, a shared audio module (in `core/domain` or a small crate), `recording_cpal_it`, the chunk seam tests.
Acceptance bar: the callback body has no allocation and no lock (asserted by a test with a counting allocator in debug, or by review of the generated code noted in design); the seam corpus (`chunking_seam_cases.rs`) and `e2e-seams` pass; a 60 s recording under load loses no samples (integration test with a synthetic stream).
Out of scope: the chunking algorithm; the recorder port.

## Capabilities

### Modified Capabilities

- `speech-recognition`: a recording loses no audio under load, the audio sent to the service is exactly what was recorded, and an audio device error ends the dictation with a notice.
- `failure-notices`: a microphone lost during a dictation is announced.

## Impact

- `app/src/adapters/secondary/recording/cpal/**`, `app/src/adapters/secondary/commandspotting/websocket/capture.rs`, a new shared module, `app/Cargo.toml` (ring buffer crate, pinned), `app/tests/recording_cpal_it.rs`, `chunk_cuts_it.rs`.

## Open points

- Ring buffer crate: `rtrb` (wait-free, used by audio projects) versus `ringbuf`; proposed: `rtrb`.
- Where the shared audio module lives; resolved in design: `app/src/adapters/secondary/recording/cpal/` (`capture.rs`, `resampler.rs`), since both microphone users are in `app` and the command spotting capture already imports `recording/cpal/devices`; no new crate.

## Release

SemVer: PATCH — fewer dropouts, a device error reported.
Target version: 0.36.3 (next PATCH above `main` at merge; `kit version-check` corrects it).
