## Why

The smoke scenario "Requests wait their turn when every copy is busy" (`e2e-test/tests/features/local_services.feature`, `@smoke @serial`) fails now and then with "the second request was answered before the first one". The speech recognition service for dictation runs as two copies, both busy with long speech; two more requests arrive one after another, and each must start on the first copy that frees, in the order they arrived, with no copy transcribing two requests at once. It failed on three branches:

- `feat/whisper-copy-pool`, 2026-10-20 16:41 (`just e2e-smoke`);
- `chore/rerun-only-failed-tests`, 2026-10-21 03:30 (`just full-check`);
- `fix/match-late-confirmation-to-its-state`, 2026-10-21 03:52 (`just full-check`), whose change does not touch the copies.

Each failure blocks a merge and costs a full check of about 1.5 hours with the lock wait.

## What Changes

- A request that waits for a busy copy keeps its place in the queue when its wait for a copy runs out and it tries again; today it leaves the queue for a short pause and comes back last, so a later request takes the copy that frees in that pause.
- The scenario checks the order in which the waiting requests start on a copy, as its promise says, not the order in which they are answered (two copies answer at slightly different speeds).

Current stage: PoC. Target stage: PoC.
Scope: the client's queue for the copies (`copy-pool/` and the transcription client) and the scenario's order check; both were wrong.
Acceptance bar: the scenario passes 5 `just e2e-smoke` runs in a row; its promise (arrival order, one request per copy) stays as it is.
Out of scope: the number of copies; other `local_services` scenarios.

## Capabilities

### Modified Capabilities

- `local-services`: waiting requests reach the copies in the order they arrived.

## Impact

- `copy-pool/` (the queue for the copies) and `app/src/adapters/secondary/transcription/openai/`
- `e2e-test/tests/features/local_services.feature` and its steps in `e2e-test/tests/steps/local_services/`

## Release

SemVer: PATCH — bug fix in request ordering or its test.
Target version: 0.30.1.
