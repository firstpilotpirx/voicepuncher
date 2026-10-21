## Why

The smoke scenario "Requests wait their turn when every copy is busy" (`e2e-test/tests/features/local_services.feature`, `@smoke @serial`) fails now and then with "the second request was answered before the first one". The speech recognition service for dictation runs as two copies, both busy with long speech; two more requests arrive one after another, and each must start on the first copy that frees, in the order they arrived, with no copy transcribing two requests at once. It failed on three branches:

- `feat/whisper-copy-pool`, 2026-10-20 16:41 (`just e2e-smoke`);
- `chore/rerun-only-failed-tests`, 2026-10-21 03:30 (`just full-check`);
- `fix/match-late-confirmation-to-its-state`, 2026-10-21 03:52 (`just full-check`), whose change does not touch the copies.

Each failure blocks a merge and costs a full check of about 1.5 hours with the lock wait.

## What Changes

- Requests that wait for a busy copy of a speech recognition service start in the order they arrived, in every run; the scenario no longer fails by chance.

Current stage: PoC. Target stage: PoC.
Scope: whether the order breaks in the service (HAProxy queueing to the copies) or in the scenario (how it tells "arrived" and "answered" apart, e.g. a second request that arrives before the first reaches HAProxy, or a shorter clip answered sooner), and fixing the one that is wrong.
Acceptance bar: the scenario passes 5 `just e2e-smoke` runs in a row; its promise (arrival order, one request per copy) stays as it is.
Out of scope: the number of copies; other `local_services` scenarios.

## Capabilities

### Modified Capabilities

- `local-services`: waiting requests reach the copies in the order they arrived.

## Impact

- the HAProxy configuration of the speech recognition services in `justfile`
- `e2e-test/tests/features/local_services.feature` and its steps in `e2e-test/tests/steps/local_services/`

## Open points

- Whether the service or the test is wrong is not known yet; proposed: read the HAProxy log of a failing run first, in design.
- Target version: proposed 0.27.2, the PATCH above 0.27.1.

## Release

SemVer: PATCH — bug fix in request ordering or its test.
Target version: 0.27.2.
