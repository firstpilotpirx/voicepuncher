# Design

## Context

The client in `app/src/adapters/secondary/commandspotting/websocket/client.rs` pings the service every second and counts the service gone once nothing came back for 5 s, measured from the last message it read. The streaming worker calls `keep_alive` (judge, then ping) before `read_events`. A worker that was busy for seconds (resampling a large burst of audio, or a sleeping machine) judges before it reads the pongs already waiting in the socket, and measures from the last message it read rather than from when it asked. Reproduced under CPU load in explore (proposal § Cause).

## Goals / Non-Goals

**Goals:** a pause of the app's own never counts as the service's silence; a service that really answers nothing for 5 s after a ping is still noticed within about 5 s.

**Non-Goals:** a cheaper resampling of large bursts; the reconnect and outage handling; the service side.

## Decisions

- **Silence counts from the oldest unanswered ping.** The client keeps the moment it sent the first ping still without any answer; any message of the service clears it. The service counts as gone when that ping is older than the 5 s limit. Rejected: counting from the last message read (today's rule), which charges the app's own pause to the service whenever no ping was out before the pause.
- **Judge only after reading.** `keep_alive` only sends the ping when one is due; the silence check moves after the reads, in `read_events` and in each round of `events_for_sent_audio`, so whatever arrived during a pause is read before anything is judged. Rejected: swapping the two calls in the worker only — `events_for_sent_audio` and every other caller would keep the wrong order.
- **The first ping goes out at the first `keep_alive`, not a second after connecting**, so a service that hangs from the start is still noticed about 5 s after connecting, as the existing hang test expects.

## Test levels

| Scenario | Level |
|---|---|
| A confirmation that takes longer than 10 seconds keeps listening on | E2E `.feature` (existing, unchanged) |
| A pause of the app's own does not lose the service | adapter integration, `app/tests/commandspotting_client_it.rs`, stub service answering pings, client paused 6 s with a ping out and with none out |
| A burst of audio after long listening does not lose the service | adapter integration against the real service, the two long-listening tests in `app/tests/commandspotting_websocket_it.rs` (`e2e-smoke`); the busy machine is not reproducible in a test, so the acceptance is repeated `e2e-smoke` runs |

## Risks / Trade-offs

- A service that dies while the app is paused is noticed right after the pause, not 5 s later: the ping out before the pause is already older than the limit. That is the right answer, only earlier.
