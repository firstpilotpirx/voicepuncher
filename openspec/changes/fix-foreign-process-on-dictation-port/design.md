## Context

The service manager probes a port over HTTP (`GET /health`, `GET /` naming whisper) before it adopts a service, but the copy pool (`copy-pool/`) probes over bare TCP and takes the service's own port alone whenever it accepts a connection. The copy ports are right: copy *n* is at port + 10000 + *n*.

## Decisions

### The copy pool identifies a port once, when it joins
A port that accepts a TCP connection is asked `GET /` once over HTTP/1.1 (1 s limit); it joins only when the answer is an HTTP/1.x `200` whose body names whisper — the check `HttpProbe` of the service manager already uses. A definite answer is remembered while the port keeps accepting connections and forgotten when it stops, so a restarted port is asked again; no answer within the limit leaves the port unidentified and asked again next round. Liveness stays the cheap TCP probe every second. Asking on every round was rejected: a copy decoding under load answers HTTP late and would leave, which the two-miss rule exists to prevent. The check sits behind the `Probe` trait (`serves(port)`, `true` by default), so the command spotting pool (WebSocket copies) keeps TCP alone; `CopyPool::shared` (speech recognition) uses the HTTP check.

### The holder of a foreign port is named from `lsof`
For a port that answered but not as whisper, the pool runs `/usr/sbin/lsof -nP -iTCP:<port> -sTCP:LISTEN -Fpc` once and logs one warning with the port, the program's name and its pid; without an answer it names "an unknown program". No new dependency, no `unsafe`.

### The cause travels as a typed error
`PoolError::NoCopies` gains the foreign program on the service's own port, if any (copy-pool 0.3.0 → 0.4.0, a breaking change of a public enum). The transcription adapter turns it into the domain error `ServicePortTaken` (port, program, pid), carried inside the `anyhow::Error` the `Transcriber` port already returns; the pipeline downcasts it to pick the failing part. `FailingPart` gains `ServicePortTaken(ForeignProgram)`, whose cause reads "port 8081 is held by app (pid 28344), not by speech recognition" and whose pill name stays "STT". A program name is a `String`, so `FailingPart` and `FailureNotice` lose `Copy` and keep `Clone`. The live worker keeps the cause of its last lost chunk next to the count.

### Test stand-ins answer the identity question
Every hand-written stand-in for a speech recognition copy answers `GET /` with a page naming whisper and does not count it as a request, through one helper in `test-steps`.

## Test levels

| Scenario | Level |
|---|---|
| Three copies are found from the service address | E2E (unchanged) |
| A single copy on the service's own port is found | E2E (unchanged) |
| A copy that came back is used again | copy-pool unit (unchanged) |
| A copy missed by one slow probe stays | copy-pool unit (unchanged) |
| A copy missed by two probes in a row leaves | copy-pool unit (unchanged) |
| A retry after a hung request reaches the other copy | adapter integration (unchanged) |
| Another program on the service's own port is passed over | copy-pool unit (scripted probe), adapter integration (`transcription_openai`, stand-in copies and a foreign HTTP/2-like listener), E2E `local_services.feature` (real copies on the test ports, a foreign listener on the service's own port) |
| Another program on a copy's port is passed over | copy-pool unit, adapter integration |
| A program on the dictation port is logged once | adapter integration (log capture) |
| Another program holds the dictation port and no copy runs | adapter integration (the error names port, program, pid), component `failure_notices.feature` (stand-in transcriber failing with `ServicePortTaken`) |
| Nothing listens on the dictation port and no copy runs | component `failure_notices.feature` |

## Risks

- A whisper-server still loading its model may not answer `GET /` within 1 s: it stays unidentified and joins on a later round, as it would only answer requests after loading anyway.
- `lsof` can take a few hundred milliseconds; it runs on the probe thread once per foreign port.
