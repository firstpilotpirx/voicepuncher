## Why

A check that fails late wastes the minutes before the failure and keeps the machine-wide lock for them. Research of 2026-10-18 (report https://claude.ai/artifact/HaDTCo6LEo833xZijMqCh7, 140 recorded check runs of 14–18 Oct): 294 of 954 minutes went to runs that failed; `e2e-smoke` failed 52 times, `e2e-seams` 10 times. Inside `e2e-smoke` the 5 `*_it` binaries run first one after another, then the ~250 s run of 178 Gherkin scenarios; the most failing tests are a few `*_it` tests (`voice_forms_it` 26 failures, `voice_samples_it` 10, `commandspotting_websocket_it` several). Short stages like `refine-eval` (≈ 25 s) run last. The user agreed (2026-10-18): run fast and often failing stages first.

## What Changes

- The model stages and the suites inside them run in an order that shows a failure soonest: short stages and the tests that failed most often in recent runs first, the long Gherkin runs (`@smoke` scenarios, seams, resilience, the 100-recording accuracy sample) after them.
- Within a stage, the test binaries and scenarios most likely to fail run first, and a stage stops at its first failure when the check will fail anyway.
- Every stage and test still runs on a passing check; only the order changes.

Current stage: PoC. Target stage: PoC.
Scope: `justfile` (`check`, `full-check`, `_stages`, `_model-stages`, `_e2e-smoke` order of `e2e_smoke_binaries`), `.config/nextest.toml` (fail-fast per suite), AGENTS.md § How to verify a change.
Acceptance bar: a check with one failing `*_it` test or a failing `refine-eval` fails within the first ~2 minutes of its model phase instead of after `e2e-smoke`'s Gherkin run; a passing check runs every stage it ran before; stage tables before and after in the PR.
Out of scope: dropping or shortening stages (`speed-up-tests`), reusing passed stages (`reuse-passed-check-stages`), the lock split (`split-machine-lock`), flaky tests (`fix-flaky-tests`).

## Capabilities

### Modified Capabilities

- none: test tooling only; no product capability changes.

## Impact

- `justfile`, `.config/nextest.toml`, AGENTS.md § How to verify a change.

## Open points

- Fixed order or one learned from recent failures: proposed — a fixed order now (fastest first, then most often failing), revisited after `fix-flaky-tests` lands.
- Stopping at the first failure: proposed — yes inside `check` and `full-check` (nextest `fail-fast` per suite), no in standalone E2E recipes used for debugging, where seeing every failure helps.
- Target version: proposed no release — tooling only.

## Release

SemVer: no release — test tooling only; no behavior change.
Target version: 0.18.0.
