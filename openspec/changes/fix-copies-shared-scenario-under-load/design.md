# Design

## Context

See proposal.md § Why. The scenario sends its 9 requests through the app's own client (`OpenAiTranscriber`), whose deadline is a formula of the audio length (5 s + 0.5 s per audio second) capped by `timeout_secs`: a short utterance gets about 5.4 s whatever the configuration says. `copies.rs` (659 code lines), `openai/mod.rs` (357) and `local_services.rs` (538) are listed in `app/tests/size_baseline.toml` and may only shrink. The Rule already carries `@serial`: the proposal's second open point needs no change.

## Goals / Non-Goals

**Goals:** the scenario proves the sharing on a loaded machine; a failure says what the machine was busy with; the listed files shrink.

**Non-Goals:** the production deadline formula; the copy pool; the other local-services scenarios.

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| Waiting longer under load | `OpenAiTranscriber::with_request_deadline(Duration)`, a builder like `with_recovery_wait`: every request takes that deadline instead of the formula's; the step uses 30 s (3 queued short transcriptions at up to 10 s each). The deadline code (`request_deadline`, `longest_wait`, constants, unit tests) moves to `openai/deadline.rs`, so `mod.rs` shrinks | the step keeps the app's client with its lease, retries and log lines; every other client keeps the formula | posting through the copy pool in the step (duplicated code); fewer requests (weaker check); a longer `timeout_secs` (the formula caps below it) |
| Load report | `unanswered_copies::load_beside` over the unanswered requests' failure lines, which carry the client's "copy N sent no answer in …" text, with `machine_load::report` | exists and is tested; names copies and busiest processes in one line | a new reporter |
| Loading the machine in the busy scenario | a `BusyProcessor` guard spawns one `yes >/dev/null` per logical core and kills them on drop, held by `PoolRun` | reproducible, independent of other sessions; a copy's encoding and queueing are CPU-bound | another session's run (not reproducible); a second whisper service (ports, minutes) |
| Making a request miss in the report scenario | the same requests with a 50 ms deadline; the Then step runs the sharing check and inspects its error text | deterministic; exercises the real failure path | killing a copy mid-request (another scenario) |
| Where the new step code lives | `copies/shared.rs`, a submodule of `copies.rs` with the shared-requests steps moved there (`transcribe`, `unanswered_at_once`, `ensure_all_answered`, `requests_at_once`, `shared_by_copies`) plus the new steps and the guard | moving ~70 lines out pays for the `mod` line; `local_services.rs` stays untouched | a sibling module under `local_services/` (grows `local_services.rs`) |

## Test levels

| Spec scenario | Level |
|---|---|
| Requests that arrive at once are shared by the copies | E2E `.feature` (unchanged) |
| Requests that arrive at once are shared by the copies while the machine is busy | E2E `.feature`, `@smoke @serial` |
| A request without a transcript names the machine's load | E2E `.feature`, `@smoke @serial` |
| Requests wait their turn when every copy is busy | E2E `.feature` (unchanged) |
| A request whose wait ran out keeps its place | E2E `.feature` (unchanged) |
| the fixed deadline of the client builder | unit, `openai/deadline.rs` |

## Risks / Trade-offs

- [30 s runs out on a machine busier than the CPU load simulates] → the failure names the busiest processes and lock holders; the bar of 5 passes beside `just e2e-regression` measures it.
- [Two more rounds of 9 requests lengthen `e2e-smoke-services` by 10–40 s] → its limit in `.config/nextest.toml` is checked against twice the new longest run.

## Dependencies

None: `fix-dictation-smoke-and-recording-test-under-load`, which filed this change, is merged (850f021e); its `load_beside` reporter is reused as is.
