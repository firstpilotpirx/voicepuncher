# Proposal

## Why

The first dictation after a pause of two minutes or more loses its text refinement and wastes time on transcription. The user's log:

```
WARN …transcription::openai] speech recognition service dropped the connection (attempt 1): …: Network Error: Connection reset by peer (os error 54) — retrying in 200ms
WARN …text_refiner] refinement failed, falling back to raw transcript: …/v1/chat/completions: Network Error: Connection reset by peer (os error 54)
…stop→paste: … refine=0ms …
```

Both services are healthy. Both adapters keep their HTTP connection open between requests (`ureq` connection pool), while `whisper-server` and `llama-server` close an idle connection after 5 seconds (`Keep-Alive: timeout=5`). Before reusing a pooled connection, the HTTP client (`ureq`, see design) peeks at it: after 20–60 seconds the peek sees the close and `ureq` silently opens a new connection, but after 120 seconds or more macOS answers the peek with `Connection reset by peer`, and `ureq` fails the request instead of opening a new connection (`unit.rs`, `connect_socket`: `stream.server_closed()?`). Reproduced on both running services: idle 20 s and 60 s → close seen; idle 120 s and 240 s → reset.

Effect on every dictation after a long pause:
- speech recognition retries after 200 ms (the retry for a busy service), so the paste comes ~200 ms later;
- text refinement has no retry and pastes the raw transcript.

Keeping the connection is worth nothing on loopback: measured on `llama-server`, a new connection costs 0.13 ms; a request over a new connection takes 0.31 ms against 0.15 ms over a reused one.

## What Changes

- Both adapters open a new connection to their local service for every request and keep no idle connection between requests.
- The retry of a dropped connection to the speech recognition service stays, for a service that is really busy.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `speech-recognition`: new requirement — a recording after a long pause reaches the service on the first attempt.
- `text-refinement`: new requirement — a transcript after a long pause is refined, not pasted raw.

## Impact

- **Stage:** current PoC → target PoC.
- **Version:** PATCH, target `0.5.3` (from 0.5.2).
- **Scope:** the `ureq` agents of `OpenAiTranscriber` (`app/src/adapters/secondary/transcription/openai/mod.rs`) and `OpenAiTextRefiner` (`app/src/adapters/secondary/textrefinement/openai/mod.rs`); their adapter integration tests.
- **Acceptance bar:**
  - Adapter integration: a stub service that resets the connection left from the previous request; the next transcription succeeds with no dropped attempt logged, and the next refinement returns the refined text.
  - `just full-check` passes.
- **Out of scope:** the retry schedule of speech recognition; a retry for text refinement; the startup checks of both services; the `ureq` version.
- **Manual run:** none (hotkey, recording, overlay, paste and permissions are unchanged).
