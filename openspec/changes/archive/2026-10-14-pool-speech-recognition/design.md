# Design

## Context

- `start-model-servers` (keep-services-running) runs each service under a bash `supervise` loop: restart with a back-off, the restart noted in the service's log, `TERM` stops the service and the loop; `start_service` waits for readiness and reports a service that exits before it was ready. PID files `~/.voicepuncher/run/<binary>-<port>.pid` hold the supervisor's PID; logs `~/.voicepuncher/logs/<binary>-<port>.log`.
- `whisper-server` serves one request at a time (research/README.md); two to four copies are about twice as fast on long speech.
- A prototype (2 copies behind HAProxy 3.4, `leastconn`, `maxconn 1`, `set-priority-class` on `Content-Length`) finished a short request queued after a long one at 2.2 s and the queued long one at 4.1 s.
- The `local_services` E2E scenarios find the speech recognition process on the service's port and read its log there.

## Goals / Non-Goals

**Goals:** several copies behind the service's address, one number to set them, a queue instead of a busy copy, short speech first, the same start, stop and restart behavior as the other services.

**Non-Goals:** detecting a copy that hangs with its port open; copies of the text refinement service; process-compose or launchd.

## Decisions

1. **Copies under the existing supervisor, HAProxy in front.** `start_whisper` starts each copy with `start_service whisper-server <copy port> …`, then writes the HAProxy configuration and starts `haproxy -db -f <config>` with `start_service haproxy <service port> …`, so HAProxy is supervised too. *Alternatives:* process-compose (`replicas: N`) — not in Homebrew core, only in its author's tap; a Rust pool service — 500–800 lines that HAProxy and the supervisor already cover; llama-swap — returns 429 instead of queueing.
2. **Number of copies.** `VOICEPUNCHER_STT_INSTANCES`, default 3 (two to four are equally fast on long speech, three gives one spare when a copy restarts). Each copy gets `-t max(1, cores / copies)`. With 1, `whisper-server` runs on the service's port as before and HAProxy is not needed; a copy list left from an earlier start is removed, so the stop finds the single service.
3. **Ports.** Copies listen on `service port + 10000 + i` (8081 → 18082, 18083, 18084; i from 1), or `service port − 10000 + i` when that would pass 65535. The list goes to `~/.voicepuncher/run/whisper-server-<service port>.copies`, which `stop-model-servers` reads.
4. **HAProxy configuration**, generated into `~/.voicepuncher/run/haproxy-<service port>.cfg` and checked with `haproxy -c` before the start:
   - `balance leastconn`, `maxconn 1` per copy: a request goes to a free copy, the others wait in the backend queue (`timeout queue` 300 s, as long as the longest chunk may take).
   - `check inter 1s fall 2 rise 1` and `observe layer4 error-limit 1 on-error mark-down`: a copy is marked down at its first refused connection, not only by the next health check, and gets no requests while its supervisor restarts it; `retries 3` with `option redispatch 1` sends a request that could not reach a copy to another one. Without the passive check, 3 of 4 requests sent right after a copy died failed.
   - `http-request set-priority-class int(-1)` when `Content-Length` is under 160 000 bytes (5 s of 16 kHz 16-bit audio plus the form fields): command confirmations (≤ 3 s) go ahead of dictation chunks in the queue.
   - `option http-server-close`: a copy's connection closes after each answer, so an idle keep-alive connection never counts against its one slot.
   - `global maxconn 256`.
5. **Stopping.** `stop_service whisper-server` stops HAProxy's supervisor (PID file `haproxy-<service port>.pid`) and every copy listed in the `.copies` file, then removes the list and the configuration. A single copy started with 1 or by an older justfile is stopped as before.
6. **Readiness and failure.** The service is ready when HAProxy answers on the service port after every copy answered on its own. A copy that fails to start is reported by `start_service` as today, with its own log; the copies already started and HAProxy are stopped, so nothing is left.
7. **`require_binary haproxy haproxy`** only when more than one copy is asked for.

## Test levels

| Spec scenario | Level |
|---|---|
| local-services / Requests that arrive at once are shared by the copies | E2E `.feature` (`local_services.feature`, `@smoke @serial`): default number of copies on free ports; each copy's log shows a transcription |
| local-services / One copy of the speech recognition service dies | E2E `.feature`: `SIGKILL` to one of two copies; four requests sent at once right after it all get a transcript; the copy's port answers again within 30 s of the kill |
| local-services / A short utterance overtakes long speech that waits | E2E `.feature`: two copies, four long requests (two busy, two waiting) then a short one; the short one finishes before the first waiting long one (fails without the priority rule) |
| local-services / The user stops the speech recognition service that runs as several copies | E2E `.feature`: nothing answers on the port, no `whisper-server` or `haproxy` process of it is left, the copy list and the configuration are removed |
| local-services / The second of several copies cannot start | E2E `.feature`: two copies, the second copy's port held bound without listening, so copy 1 is up when copy 2 fails; the report names copy 2's log; copy 1 (and its supervisor), HAProxy and the copy list are gone |
| local-services / The user stops a single speech recognition service after copies were left listed | E2E `.feature`: a stale copy list, a one-copy start, a stop; nothing answers and no process is left |
| existing local-services scenarios | unchanged meaning; their steps start one copy |

## Risks / Trade-offs

- [More GPU memory: three model copies (small ≈ 0.5 GB each)] → fine on the target Macs; the number is one setting.
- [A copy hangs with its port open] → its requests wait until HAProxy's server timeout; out of scope (proposal).
- [HAProxy becomes a requirement] → Homebrew core formula, installed like `whisper-cpp`; one copy needs no HAProxy.
- [The user's running services started by an older justfile] → `start-model-servers` skips a port that answers, as today; `just run` restarts them, which brings up the copies.
