# Design

## Context

`OpenAiTranscriber` posts one multipart request per chunk through a `ureq` 2 agent and maps any error to a failed cycle. `ureq` 2 never retries a POST with a body, and it already discards pooled connections the server closed while idle (it peeks before reuse), so the failure is not a stale keep-alive connection: it is the service resetting connections it has no worker for. In the failing log the 19.6 s chunk never reached `whisper-server` (absent from its log), while about a hundred E2E requests from another checkout were processed in the same window.

## Goals / Non-Goals

**Goals:** a transient connection drop does not lose the recording; the delay added to a healthy cycle is zero.

**Non-Goals:** retrying HTTP errors or timeouts; changing the `Transcriber` port.

## Decisions

1. **Retry inside the adapter, not in the pipeline.** Whether an error is a transport-level drop is only known to the HTTP adapter; the pipeline keeps its "a failure drops the cycle" rule. Alternative — a retry in `pipeline.rs` around `transcribe` — would also retry HTTP 500s and timeouts, which the spec forbids.
2. **What counts as a dropped connection:** a `ureq::Error::Transport` whose kind is `ConnectionFailed`, or `Io` with an underlying `io::ErrorKind` of `ConnectionRefused`, `ConnectionReset`, `ConnectionAborted`, `BrokenPipe` or `UnexpectedEof`. A `TimedOut`/`WouldBlock` I/O error is a timeout and is not retried; a `ureq::Error::Status` is an error answer and is not retried.
3. **Schedule:** 3 retries after 200, 400 and 800 ms (1.4 s total). The live worker transcribes chunks one by one, so this blocks only that chunk. Each retry logs a `warn` with the attempt number and the error.
4. **E2E concurrency:** `E2eWorld::cucumber().max_concurrent_scenarios(4)`. Four concurrent clients stay well below the observed limit (one of 12 concurrent connections was reset), leaving room for the user's live session. Alternative — a separate service pair for E2E — doubles the model memory (~2.5 GB); rejected.

## Test levels

| Spec scenario | Level |
|---|---|
| Service is briefly busy | adapter integration (`app/tests/transcription_openai_it.rs`, stub resets the first two connections) |
| Service keeps dropping the connection | adapter integration (stub resets every connection, counts 4 attempts) |
| Service answers with an error | adapter integration (stub answers HTTP 500, counts 1 attempt) |

The stub resets a connection by closing it without reading the request body, which makes the OS send RST — the same effect the busy service has.

## Risks / Trade-offs

- [A service that is really down now fails 1.4 s later] → acceptable: the cycle is dropped anyway; the startup check still reports a missing service immediately.
- [A request the service had already started may run twice] → only when the connection drops before any answer; transcription has no side effects.
