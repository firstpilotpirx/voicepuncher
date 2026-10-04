# Proposal

## Why

`should_retry_when_service_drops_connection` and `should_give_up_after_four_attempts_when_service_keeps_dropping` (`app/tests/transcription_openai_it.rs`) fail in about 1 run of 15 on macOS. The tests are right; the adapter is wrong. When the service resets the connection after the recording was sent, macOS sometimes reports the reset as `Invalid argument (os error 22)` instead of `Connection reset by peer` when the client starts reading the answer:

```
speech recognition request to http://127.0.0.1:63040/v1/audio/transcriptions failed: …
Network Error: Error encountered in the status line: Invalid argument (os error 22)
```

The retry from 0.3.2 (`fix-busy-speech-service`) does not count this error as a dropped connection, so it drops the recording at once — the same loss of a dictation that 0.3.2 fixed, now on a timing-dependent path. Measured: 4 failures in 60 runs of the integration test binary.

## What Changes

- An operating-system "invalid argument" error on the connection, raised while the request is sent or the answer is read, counts as a dropped connection and is retried like a reset.
- Other errors are classified as before: timeouts and error answers are not retried.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `speech-recognition`: "Dropped connection to the service is retried" gains a scenario where the drop surfaces as a different connection error than a reset.

## Impact

- **Stage:** current PoC → target PoC.
- **Version:** PATCH: `0.4.0` → `0.4.1`.
- **Scope:** `is_dropped_connection` / `is_dropped_io` in `app/src/adapters/secondary/transcription/openai/mod.rs` and their unit tests.
- **Acceptance bar:**
  - Unit: an OS `EINVAL` I/O error is a dropped connection; an `InvalidInput` error raised by a library (no OS error code) is not.
  - The integration test binary `transcription_openai_it` passes 200 runs in a row on macOS.
  - `just full-check` passes.
- **Out of scope:** the retry schedule and attempt count; the stub in the integration tests; other flaky tests.
- **Manual run:** none (hotkey, recording, overlay, paste and permissions are unchanged).
