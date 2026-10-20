# Design

## Context

`OpenAiTranscriber` (`adapters/secondary/transcription/http/`) posts chunks with ureq to `[transcription] base_url` and retries on dropped connections; the live chunk worker sends chunks as they close. `UtteranceTranscriber` in `command-spotting/src/confirm.rs` posts short utterances with a deadline. HAProxy gives both one address for three copies, leastconn with maxconn 1, a priority class for short requests, abortonclose, TCP health checks. The `justfile` computes copy ports as base + 10000 + i (or base − 10000 + i past 65535). The `local-services` spec has 15 requirements written for the scripts.

## Decisions

**D1 — A workspace crate, not a module of the app.** The pool is needed by two binaries (the app and the command spotting service); a crate `copy-pool/` with only `log` as a dependency is the smallest shared unit and keeps `core` free of it (adapters use it). Alternative: a module in `command-spotting` re-exported — rejected, the app must not depend on the service's crate.

**D2 — Discovery by the existing port convention.** The `justfile` already places copies at base + 10000 + i (base − 10000 + i when that would pass 65535; the pool applies the same rule). When the base port itself answers, it is the only copy and the convention ports are not used: the `justfile` never runs both, and tests with a fake server on a random base port would otherwise take another test's fake server on base + 10001 for a copy; making the clients know the same rule needs no config key, no file, no protocol, and `local-services-runtime` can later pass the exact list. Probing nine ports every second with a 200 ms connect timeout is negligible on loopback.

**D3 — One request per copy, FIFO waiting on a condvar.** This is HAProxy's `maxconn 1` + queue in ten lines; it keeps whisper-server from queueing internally where nothing can see or abort a request. One pool per process per service address: `CopyPool::shared(base_url)` hands every client of a process the same pool (a registry keyed by the address), because the E2E steps and the app's tests build a client per request, and nine clients with a pool each would all take the first copy. Alternative: a pool per client — rejected for that reason.

**D3a — A per-process start offset, no cross-process coordination.** Processes against the same service (the accuracy benchmark runs two app processes at once) cannot see each other's leases; each process starts its search for a free copy at its process id modulo the number of copies and goes round, so two processes usually land on different copies and whisper-server rarely queues internally. Alternative: a lock file or a shared memory map per copy — rejected: the only case is the benchmark, and a collision costs one queued request, not a failure.

**D3b — The live worker stays sequential.** The app sends the chunks of one dictation one after another (one `Transcriber`, `&mut self`); the pool gives dictation resilience (a dead copy skipped, a chunk retried on another), not parallel chunks. Making the worker send chunks concurrently reorders the live transcript and is a change of its own.

**D4 — Abandonment by finishing the read, not by closing the socket.** The caller stops waiting at its deadline (a channel receive with timeout), but the request thread keeps reading the response and frees the copy only when it arrives: the copy is naturally "draining". This replaces `abortonclose` and needs no second connection. A copy that answers nothing within twice the timeout is reported hung and marked down until a probe sees it answer again.

**D5 — Retries take a new lease.** Every new attempt of the app's existing retries (3 more after a dropped or refused connection, 2 more after a timeout or an error answer) takes a new lease: a copy that refused the connection is marked down, and a copy still working on an attempt the app gave up on is draining, so the next attempt goes to another copy or waits — `retries 3` + `redispatch` without the proxy. The retry counts of the `speech-recognition` spec stay as they are; the spec only adds that a new attempt goes to another copy. Alternative: no retry after a timeout — rejected: the spec's "first request of a chunk hangs" scenario needs it, and with draining the retry no longer doubles the load on the hung copy.

**D6 — The short-first priority is removed.** Commands have their own service since the `commands-service` change; a priority class across one pool has nothing to order. The spec requirement is removed, not rewritten.

**D7 — The `justfile` keeps supervising copies.** Starting, health-watching and restarting copies stays in the scripts until `local-services-runtime`; this change touches only the routing.

## Risks / Trade-offs

- [Chunks of one dictation on different copies lose the per-copy prompt cache] → HAProxy already spread them; no change in accuracy expected; `full-check` on 100 recordings measures it.
- [A single copy on the base port and the convention ports both answering] → impossible by the `justfile`; the pool treats every answering port as a copy anyway.
- [Probing from two processes (app and command spotting) doubles the probe load] → 18 connects per second on loopback; negligible.

## Test levels

| Scenario | Level |
|---|---|
| Three copies are found from the service address | E2E `local_services.feature` (`@smoke`) |
| A single copy on the service's own port is found | E2E `local_services.feature` (`@smoke`) |
| A copy that came back is used again | unit test in `copy-pool` (fake probe) |
| A copy dies as a request reaches it | adapter integration `transcription_openai_it.rs` (fake copies) |
| Every copy refuses the request | adapter integration `transcription_openai_it.rs` |
| A copy never answers a request | unit test in `copy-pool`; adapter integration `transcription_openai_it.rs` |
| Nine requests from one process spread over three copies | unit test in `copy-pool` (registry); E2E "Requests that arrive at once are shared by the copies" |
| Two processes start with different copies | unit test in `copy-pool` (start offset) |
| Requests that arrive at once are shared by the copies | E2E `local_services.feature` (`@smoke`) |
| Requests wait their turn when every copy is busy | unit test in `copy-pool`; E2E `local_services.feature` (`@smoke`) |
| The user stops the speech recognition service that runs as several copies | E2E `local_services.feature` (`@smoke`) |
| The second of several copies cannot start | E2E `local_services.feature` (`@smoke`) |
| The request after an abandoned one goes to another copy | E2E `local_services.feature` (`@resilience`) |
| An abandoned request is not transcribed | unit test in `copy-pool`; E2E `local_services.feature` (`@resilience`) |
| Service is briefly busy | adapter integration `transcription_openai_it.rs` (existing) |
| Drop reported as an invalid argument | adapter integration `http_failure_it.rs` (existing) |
| Service keeps dropping the connection | adapter integration `transcription_openai_it.rs` (existing) |
| Service answers with an error | adapter integration `transcription_openai_it.rs` (existing) |
| A chunk whose copy is killed is transcribed by another copy | adapter integration `transcription_openai_it.rs` (fake copies, one closed) |
| The first request of a chunk hangs | adapter integration `transcription_openai_it.rs` (existing) |
| One error answer is retried | adapter integration `transcription_openai_it.rs` (existing) |
| The attempt after a hung one goes to another copy | adapter integration `transcription_openai_it.rs` (fake copies) |
| Every copy for commands is busy past the deadline | unit test in `command-spotting/src/confirm.rs` (fake copies) |
| A copy frees before the deadline | unit test in `command-spotting/src/confirm.rs` (fake copies) |
| Accuracy unchanged | `full-check` WER on 100 recordings |
| Voice command during a long dictation; a copy killed mid-dictation | manual run by the user |
