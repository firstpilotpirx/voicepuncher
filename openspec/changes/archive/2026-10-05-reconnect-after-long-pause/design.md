# Design

## Context

Both adapters build one `ureq::Agent` at startup and reuse it for every request. `ureq` keeps finished connections in a pool (default: 100 idle connections, 1 per host) and reuses them. The local services close an idle connection after 5 seconds. `ureq` 2.12.1 checks a pooled connection with a non-blocking `peek` before reuse and drops it when the peek reads the close, but returns the error when the peek fails — which macOS does with `ECONNRESET` once the connection has been idle for about two minutes.

## Goals / Non-Goals

**Goals:** no request ever goes out on a connection the service closed; no extra latency.

**Non-Goals:** a retry for text refinement; changing the speech recognition retry; upgrading `ureq` (3.x is a different API, and its pooling would need the same setting).

## Decisions

1. **Disable the connection pool in both agents** with `AgentBuilder::max_idle_connections(0)` and `max_idle_connections_per_host(0)`: `ureq` then never returns a finished connection to the pool, so every request opens a new loopback connection. Cost: ~0.15 ms per request, measured. Alternatives:
   - *Retry a reset on a reused connection* (in both adapters): every dictation after a long pause still pays one failed attempt, and text refinement would need its own retry loop.
   - *A longer keep-alive on the services*: neither `whisper-server` nor `llama-server` has a setting for it, and pauses between dictations last minutes to hours anyway.
   - *TCP keepalive on the client*: does not stop the service from closing an idle HTTP connection.
2. **The startup probes keep their own agents unchanged**: they send one request and are dropped.

## Test levels

| Scenario | Level |
|---|---|
| `speech-recognition` — Service closed the idle connection during a long pause | adapter integration, `app/tests/transcription_openai_it.rs` |
| `text-refinement` — Service closed the idle connection during a long pause | adapter integration, `app/tests/textrefinement_openai_it.rs` |

The stub answers the first request on a connection it keeps open, then resets that connection (`SO_LINGER` 0, via the `libc` dev-dependency) before the second request; the second request must succeed. With the pool, the second request goes out on the reset connection and fails, so the test is red before the fix.

## Risks / Trade-offs

- A new connection per request also means a new TCP handshake per chunk of a long dictation: ~0.13 ms each, negligible against ~0.9 s of transcription per chunk.
