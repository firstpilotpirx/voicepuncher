# Design

## Context

See proposal.md § Why. `confirm.rs` cuts a confirmation short after a fixed 1 s budget made for speech stuck repeating (1.7–1.8 s idle); under load an ordinary confirmation takes 0.5–1.2 s, so the budget cuts commands. `pool.rs` `probe_round` probes each port once a second with a 200 ms connect timeout: one slow probe drops a listening copy for a second, and the first round decides a fresh pool's copies. `transcribe` is ratcheted at cyclomatic 15: new logic goes into modules of its own.

## Goals / Non-Goals

**Goals:** a slow confirmation on a busy machine is heard in full; repeating speech is still cut short when idle; one slow probe never drops a copy, in the first round too.

**Non-Goals:** measuring load; the deadline, the number of copies, the engine's queueing; the dictation transcriber's retry policy.

## Decisions

### D1. The decoding budget follows the pace of the recent confirmations

`confirm/pace.rs`: `Pace` keeps the times of the transcriber's last 4 confirmation outcomes (an answer's time, or the budget a cut-short one was cut at), across the copies, which share one GPU. The next budget is twice the slowest of them, at least 1 s, never past the deadline. Idle answers of 0.2–0.5 s keep it at 1 s; under load the checks before a segment raise it to 2–3 s before the command's segment is confirmed, and two cut-shorts in a row raise it to the deadline.

Alternatives: re-sending a cut-short utterance (doubles the copy time of repeating speech); a higher fixed budget (loses commands under heavier load, or holds a copy for a whole repeating decode); the engine's lag (a consequence of what the transcriber sees first-hand).

### D2. A missed probe is tried again at once; a copy leaves only when both attempts miss

`Inner::probe_round` probes a port that got no connection once more before the round goes on; a copy is down only when both attempts miss. In the pool, not `TcpProbe`, so the scripted probe of the pool tests covers it and a fresh pool's first round is as robust. A request's refused connection still drops the copy at once. Cost: an absent port takes 400 ms per round; three copies with the service port absent probe in about 0.8 s, within the 1 s interval.

Alternatives: a longer connect timeout (five absent ports no longer fit the interval; a longer stall still misses); two rounds in a row (the first round still decides on one probe; a dead copy takes two seconds to leave).

### D3. The "under load" scenarios run the engine against an HTTP copy that decodes slowly

`placed_commands_it.rs` runs its clocked engine with the real `UtteranceTranscriber` against a fake HTTP copy (`tests/scripted_copy/`) that parses the request's WAV, answers from `ScriptedHearing` as verbose JSON with word times, and holds each answer 1.2 s of wall time; the stream clock keeps the confirmations in their place in the speech.

## Test levels

| Scenario | Level |
|---|---|
| Russian speech that makes speech recognition repeat itself | E2E `local_services.feature` (unchanged) |
| A command right after such speech is carried out | E2E `local_services.feature` (unchanged) |
| A confirmation still decoding after 1 second is cut short | unit, `confirm/tests.rs` |
| A confirmation decoding slowly under load is heard in full | unit, `confirm/tests.rs` |
| The budget never passes the deadline | unit, `confirm/tests.rs` and `confirm/pace.rs` |
| Three copies are found from the service address | E2E `local_services.feature` (unchanged) |
| A single copy on the service's own port is found | E2E `local_services.feature` (unchanged) |
| A copy that came back is used again | unit, `pool/tests.rs` (unchanged) |
| A copy missed by one slow probe stays | unit, `pool/tests.rs` |
| A copy missed by two probes in a row leaves | unit, `pool/tests.rs` |
| A retry after a hung request reaches the other copy under load | adapter integration, `app/tests/transcription_openai_it.rs` (existing test, made robust by D2) |
| A stop right after dictated speech under load | component, `placed_commands_it.rs` (D3) |
| A chain of commands under load | component, `placed_commands_it.rs` (D3) |
| A restart after a pause under load | component, `placed_commands_it.rs` (D3) |

## Risks / Trade-offs

- [A fresh transcriber's first confirmations under load are still cut at 1 s] → the checks before a command's segment fill the window; the component tests start from a cold window.
- [Repeating speech right after a loaded spell decodes up to the deadline] → only for the four confirmations after the slow ones.
- [A round with many absent ports takes longer] → at most 2 s for five, delaying only the next round.
- [Real load cannot be reproduced in CI] → the slow fake copy reproduces the mechanism; the check of this branch runs beside other sessions.
