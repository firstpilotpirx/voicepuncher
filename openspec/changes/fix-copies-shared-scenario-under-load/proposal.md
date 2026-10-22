## Why

The E2E scenario "Requests that arrive at once are shared by the copies" (`e2e-test/tests/features/local_services.feature:57`; steps `requests_at_once` / `shared_by_copies` and `unanswered_at_once` in `e2e-test/tests/steps/local_services/copies.rs`: 9 short transcription requests sent at once to 3 copies of the dictation speech recognition service that the scenario starts on ports of its own) failed in the `e2e-smoke-services` stage of `just e2e-smoke` on branch `fix/fix-dictation-smoke-and-recording-test-under-load` (commit 3f8b95f2) on 2026-10-22 at 22:33:

```
2 of 9 requests got no transcript:
request 2: speech recognition request to http://127.0.0.1:64826/v1 failed on copies 54828: copy 54828 sent no answer in 5439ms
request 7: speech recognition request to http://127.0.0.1:64826/v1 failed on copies 54828: every copy of the service at http://127.0.0.1:64826/v1 stayed busy for 5439 ms
```

Beside it ran `just e2e-regression` from the main checkout (the light lock class: voice control on the light test ports, the command speech recognition service and the command spotting service on the GPU) and the smoke stage's own shared heavy services. With 9 requests on 3 copies, each copy queues 3 short requests, and the request deadline of about 5.4 s (5 s plus 0.69 s per audio second) covers 3 queued transcriptions only on an idle GPU. The same scenario passed in the run before it, without the regression run beside it. A flake is a failure (AGENTS.md § How to verify a change): `fix-dictation-smoke-and-recording-test-under-load`, whose acceptance bar is `just e2e-smoke` 3 times beside `just e2e-regression`, waits for this fix.

## What Changes

- The scenario passes beside a light-class run: it still proves that requests arriving at once are shared by the three copies (each copy transcribed some of them), but no longer needs 3 queued short transcriptions to fit a 5.4 s deadline on a loaded GPU.
- A failure of the scenario reports the machine's load beside it (`e2e-test/src/machine_load.rs`, as a start cue over its bound does), so the next failure says what the copies were busy with.

Current stage: PoC. Target stage: PoC.
Scope: the scenario, its steps in `e2e-test/tests/steps/local_services/copies.rs` and the `local-services` spec scenario it mirrors.
Acceptance bar: `just e2e-smoke-services` (the `e2e-smoke-services` stage) passes 5 times in a row with `just e2e-regression` running beside it from another worktree; `just test` green.
Out of scope: the production request deadline (`request_deadline` in the openai transcriber) and the copy pool; the other local-services scenarios.

## Capabilities

### Modified Capabilities

- `local-services`: the scenario "Requests that arrive at once are shared by the copies" states what it proves under load (the sharing, not a deadline) and what its failure reports.

## Impact

- `e2e-test/tests/features/local_services.feature`, `e2e-test/tests/steps/local_services/copies.rs`, `openspec/specs/local-services/spec.md`; the `e2e-smoke-services` stage of `just e2e-smoke`, `check` and `full-check`.

## Open points

- Which way the scenario holds under load, decided in design: fewer requests per copy (6 instead of 9, 2 queued each), a deadline the step itself sets for the loaded case (the step's own client, not the app's deadline), or waiting for the answers of the copies in order — proposed: the step's own deadline sized for 3 queued transcriptions on a loaded GPU, so the sharing assertion stays as it is.
- Whether the scenario should run alone (`@serial`), so no other scenario's app process queues on the copies at the same time — proposed: yes, if the measurement shows other scenarios' requests on the same copies; otherwise no.

## Release

SemVer: no release — test-only change, no behavior change.
Target version: unchanged.
