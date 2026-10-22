# Design

## Context

See proposal.md § Why. `confirm.rs` cuts a confirmation short after a fixed 1 s budget made for speech stuck repeating (1.7–1.8 s idle); under load an ordinary confirmation takes 0.5–1.2 s, so the budget cuts commands. `pool.rs` `probe_round` probes each port once a second with a 200 ms connect timeout: one slow probe drops a listening copy for a second, and the first round decides a fresh pool's copies. `transcribe` is ratcheted at cyclomatic 15: new logic goes into modules of its own.

## Goals / Non-Goals

**Goals:** a slow confirmation on a busy machine is heard in full; repeating speech is still cut short when idle; one slow probe never drops a copy, in the first round too.

**Non-Goals:** measuring load; the deadline, the number of copies, the engine's queueing; the dictation transcriber's retry policy.

## Decisions

### D1. The decoding budget follows the pace of the recent confirmations

`confirm/pace.rs`: `Pace` keeps the times of the transcriber's confirmation outcomes of the last 10 seconds (an answer's time, or the budget a cut-short one was cut at), across the copies, which share one GPU. The next budget is twice the slowest of them, at least 1 s, never past the deadline. Idle answers of 0.2–0.5 s keep it at 1 s; under load the checks before a segment raise it to 2–3 s before the command's segment is confirmed, and two cut-shorts in a row raise it to the deadline.

Alternatives: re-sending a cut-short utterance (doubles the copy time of repeating speech); a higher fixed budget (loses commands under heavier load, or holds a copy for a whole repeating decode); the engine's lag (a consequence of what the transcriber sees first-hand).

### D2. A missed probe is tried again at once; a copy leaves only when both attempts miss

`Inner::probe_round` probes a port that got no connection once more before the round goes on; a copy is down only when both attempts miss. In the pool, not `TcpProbe`, so the scripted probe of the pool tests covers it and a fresh pool's first round is as robust. A request's refused connection still drops the copy at once. Cost: an absent port takes 400 ms per round; three copies with the service port absent probe in about 0.8 s, within the 1 s interval.

Alternatives: a longer connect timeout (five absent ports no longer fit the interval; a longer stall still misses); two rounds in a row (the first round still decides on one probe; a dead copy takes two seconds to leave).

### D3. The "under load" scenarios run the engine against an HTTP copy that decodes slowly

`placed_commands_it.rs` runs its clocked engine with the real `UtteranceTranscriber` against a fake HTTP copy (`tests/under_load/scripted_copy.rs`, beside the under-load tests in `tests/under_load/mod.rs`, a submodule of `placed_commands_it.rs` so that file does not grow) that parses the request's WAV, answers from `ScriptedHearing` as verbose JSON with word times, and holds each answer 1.2 s of wall time; the stream clock keeps the confirmations in their place in the speech.

## Test levels

| Scenario | Level |
|---|---|
| Russian speech that makes speech recognition repeat itself | E2E `local_services.feature` (unchanged) |
| A command right after such speech is carried out | E2E `local_services.feature` (unchanged) |
| A confirmation still decoding after 1 second is cut short | unit, `confirm/tests/under_load.rs` |
| A confirmation decoding slowly under load is heard in full | unit, `confirm/tests/under_load.rs` |
| The budget never passes the deadline | unit, `confirm/tests/under_load.rs` and `confirm/pace.rs` |
| Three copies are found from the service address | E2E `local_services.feature` (unchanged) |
| A single copy on the service's own port is found | E2E `local_services.feature` (unchanged) |
| A copy that came back is used again | unit, `pool/tests.rs` (unchanged) |
| A copy missed by one slow probe stays | unit, `pool/tests.rs` |
| A copy missed by two probes in a row leaves | unit, `pool/tests.rs` |
| A retry after a hung request reaches the other copy | adapter integration, `app/tests/transcription_openai_it.rs` (existing test, made robust by D2) |
| A stop right after dictated speech under load | component, `placed_commands_it.rs` (D3) |
| A chain of commands under load | component, `placed_commands_it.rs` (D3) |

## Risks / Trade-offs

- [A fresh transcriber's first confirmations under load are still cut at 1 s] → the checks before a command's segment fill the window; the component tests start from a cold window.
- [Repeating speech right after a loaded spell decodes up to the deadline] → only for 10 seconds after the slow ones; the last four outcomes alone forgot a loaded spell within a second of quick answers, and the voice smoke of 2026-10-22 lost commands cut short at the floor that way.
- [A round with many absent ports takes longer] → at most 2 s for five, delaying only the next round.
- [Real load cannot be reproduced in CI] → the slow fake copy reproduces the mechanism; the check of this branch runs beside other sessions.
- [The quiet restart lost in the E2E run is not reproduced at component level] → a restart said alone survives a cut-short first hearing through the engine's second hearing of a short segment, so that loss has no component test; the budget fix covers its cut-short confirmations all the same.

## Size budgets

`confirm.rs` keeps its 300-line budget by moving the request body (`wav`, `multipart_body`, the form constants) into `confirm/request.rs`; the under-load unit tests live in `confirm/tests/under_load.rs`. `transcribe` hands its outcome to `settle`, which drops it below the cyclomatic ratchet (its entry leaves `.config/analyzers/metrics.toml`).

## Test limits

`placed_commands_it` stays under the default limit of `.config/nextest.toml` (2 min per test): its longest test after this change, the chain under load, took 8 s in `just test` on 2026-10-22, far under half the limit, so no limit changes.

## After archive: the busy-copies scenario under load

The second and third `full-check` lost the E2E scenario "Requests wait their turn when every copy is busy" (`local-services`), both with "every copy stayed busy for 5439 ms": the waiting requests of the app give a busy pool three waits of 5 s plus half their audio, about 16 s in all, and neither copy answered its long request within them. The long speech is about a minute of audio; three times it (175 s) had failed the scenario 11 times on other branches, and two times it (117 s) failed twice here while other checkouts' checks loaded the GPU. The busy speech is now one long speech (58 s): still busy when the next requests arrive 0.2–0.4 s later, decoded in a few seconds idle and well within the 16 s under load. The scenario's assertions (order, no copy takes two requests at once) are unchanged.

## Dependencies

The fourth `full-check` (commit f6c22507) passed the busy-copies scenario and lost the seams scenario "Restart restart followed by the new dictation at once gives its cue at once" (`voice-control`): the "restart restart" confirmation was answered in 1406 ms, against 145–270 ms idle, while the full-check of `fix/realtime-safe-audio-capture` ran its unlocked lint, test and build stages on the same machine, so the cue came 1810 ms after the words, past the 0.7 s the spec requires "with the services otherwise idle". The command was heard late, not missed, as this change intends; the cause is the load of another checkout's builds, outside this change. It is filed as `idle-machine-for-cue-timing-scenarios` and declared in `orchestration.yaml`, so the branch lands after it.
