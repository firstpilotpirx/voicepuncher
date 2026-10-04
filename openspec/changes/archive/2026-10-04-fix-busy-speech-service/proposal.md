# Proposal

## Why

A recording is lost when the speech recognition service is momentarily busy. The service is shared with E2E runs in other checkouts; when they open many connections at once, the service resets the connections it has no room for (`Connection reset by peer`, reproduced locally with 12 concurrent requests against `whisper-server`). The app treats that reset as a failed cycle and drops the dictation, although the same request succeeds a moment later.

## What Changes

- When the speech recognition service refuses or drops the connection before it answers, the app retries the same audio a few times with a short, growing pause (at most about 1.5 s in total) before it gives up on the cycle. Each retry is logged.
- An HTTP error answer, an unusable answer or a timeout is still not retried and still drops only that cycle, as today.
- The E2E harness runs at most 4 scenarios at a time, so a check run in one checkout no longer saturates the service the user dictates into.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `speech-recognition`: adds a requirement that a dropped connection to the service is retried before the recording is given up.

## Impact

- `app/src/adapters/secondary/transcription/openai/` — retry loop around the request.
- `app/tests/transcription_openai_it.rs` — reproduction test with a stub that resets connections.
- `e2e-test/tests/cucumber_e2e.rs` — scenario concurrency limit.
- No config, API or dependency change.

## Stage, scope and acceptance

- Current stage: PoC; target stage: PoC (unchanged).
- Scope: the speech recognition adapter and the E2E harness concurrency.
- Acceptance bar: the reproduction test (service resets the first connections, then answers) fails before the fix and passes after it; `just full-check` passes.

## Out of scope

- Keeping the audio of a cycle that still fails after the retries (for example saving it for re-transcription).
- Retries for the text refinement service: its failure already falls back to the raw transcript, so nothing is lost.
- Running a separate service instance for E2E.

## Version

PATCH: 0.3.1 → 0.3.2.
