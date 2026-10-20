# Design

## Context

`OpenAiTranscriber` (`adapters/secondary/transcription/http/`) posts chunks with ureq to `[transcription] base_url` and retries on dropped connections; the live chunk worker sends chunks as they close. `UtteranceTranscriber` in `command-spotting/src/confirm.rs` posts short utterances with a deadline. HAProxy gives both one address for three copies, leastconn with maxconn 1, a priority class for short requests, abortonclose, TCP health checks. The `justfile` computes copy ports as base + 10000 + i (or base − 10000 + i past 65535). The `local-services` spec has 15 requirements written for the scripts.

## Decisions

**D1 — A workspace crate, not a module of the app.** The pool is needed by two binaries (the app and the command spotting service); a crate `copy-pool/` with only `log` as a dependency is the smallest shared unit and keeps `core` free of it (adapters use it). Alternative: a module in `command-spotting` re-exported — rejected, the app must not depend on the service's crate.

**D2 — Discovery by the existing port convention.** The `justfile` already places copies at base + 10000 + i; making the clients know the same rule needs no config key, no file, no protocol, and `local-services-runtime` can later pass the exact list. Probing nine ports every second with a 200 ms connect timeout is negligible on loopback.

**D3 — One request per copy, FIFO waiting on a condvar.** This is HAProxy's `maxconn 1` + queue in ten lines; it keeps whisper-server from queueing internally where nothing can see or abort a request.

**D4 — Abandonment by finishing the read, not by closing the socket.** The caller stops waiting at its deadline (a channel receive with timeout), but the request thread keeps reading the response and frees the copy only when it arrives: the copy is naturally "draining". This replaces `abortonclose` and needs no second connection. A copy that answers nothing within twice the timeout is reported hung and marked down until a probe sees it answer again.

**D5 — Retries take a new lease.** A connection failure returns the lease as down and takes another (3 tries), mirroring `retries 3` + `redispatch`; a timeout is not retried (the audio was received; retrying would double the load).

**D6 — The short-first priority is removed.** Commands have their own service since the `commands-service` change; a priority class across one pool has nothing to order. The spec requirement is removed, not rewritten.

**D7 — The `justfile` keeps supervising copies.** Starting, health-watching and restarting copies stays in the scripts until `local-services-runtime`; this change touches only the routing.

## Risks / Trade-offs

- [Chunks of one dictation on different copies lose the per-copy prompt cache] → HAProxy already spread them; no change in accuracy expected; `full-check` on 100 recordings measures it.
- [A single copy on the base port and the convention ports both answering] → impossible by the `justfile`; the pool treats every answering port as a copy anyway.
- [Probing from two processes (app and command spotting) doubles the probe load] → 18 connects per second on loopback; negligible.

## Test levels

| Scenario | Level |
|---|---|
| Copies found from the address; down at first refusal; back within a second; late copy joins | unit tests in `copy-pool` (fake probe, fake clock) |
| Free copy or wait, FIFO, never two on one copy | unit tests in `copy-pool`; adapter integration `transcription_http_it.rs` (two fake servers) |
| A dead copy's request is retried on another, up to three tries | adapter integration `transcription_http_it.rs`; E2E `@resilience` (copy killed while dictating) |
| An abandoned request does not hold a copy for the next one | unit test in `copy-pool`; adapter integration (slow fake server) |
| A hung copy is reported and skipped | unit test in `copy-pool` |
| A confirmation gives up at its deadline without a copy | unit test in `command-spotting/src/confirm.rs`; E2E `@resilience` (commands service paused) |
| Copies start on the convention ports; stop stops them; no HAProxy | `just e2e-smoke` (`@local_services` steps), `full-check` |
| Accuracy unchanged | `full-check` WER on 100 recordings |
| Voice command during a long dictation; a copy killed mid-dictation | manual run by the user |
