## Why

A check that fails late wastes the minutes before the failure and keeps the machine-wide lock for them. Research of 2026-10-18 (report https://claude.ai/artifact/HaDTCo6LEo833xZijMqCh7, 140 recorded check runs of 14–18 Oct): 294 of 954 minutes went to runs that failed; `e2e-smoke` failed 52 times, `e2e-seams` 10 times. Inside `e2e-smoke` the 5 `*_it` binaries run first one after another, then the ~250 s run of 178 Gherkin scenarios; the most failing tests are a few `*_it` tests (`voice_forms_it` 26 failures, `voice_samples_it` 10, `commandspotting_websocket_it` several). Short stages like `refine-eval` (≈ 25 s) run last. The user agreed (2026-10-18): run fast and often failing stages first.

## What Changes

- The model stages and the suites inside them run in an order that shows a failure soonest: short stages and the tests that failed most often in recent runs first, the long Gherkin runs (`@smoke` scenarios, seams, resilience, the 100-recording accuracy sample) after them.
- Within a stage, the test binaries and scenarios most likely to fail run first, and a stage stops at its first failure when the check will fail anyway.
- Every stage and test still runs on a passing check; only the order changes.

Current stage: PoC. Target stage: PoC.
Scope: `justfile` (`check`, `full-check`, the order of the smoke parts and of the voice control part's binaries, the E2E recipes' nextest calls, the stage keys), `e2e-test` harness (cucumber fail-fast), AGENTS.md § How to verify a change.
Acceptance bar: a check with one failing voice control `*_it` test fails within the first ~2 minutes of its light phase, a failing `refine-eval` within the first ~2 minutes of its heavy phase, instead of after the long Gherkin runs; a passing check runs every stage it ran before; stage tables before and after in the PR.
Out of scope: dropping or shortening stages (`speed-up-tests`), reusing passed stages (`reuse-passed-check-stages`), the lock split (`split-machine-lock`), flaky tests (`fix-flaky-tests`).

## Capabilities

### Modified Capabilities

- none: test tooling only; no product capability changes.

## Impact

- `justfile`, `e2e-test/src/harness_args.rs`, `e2e-test/tests/cucumber_e2e.rs`, AGENTS.md § How to verify a change.

## Open points

- Settled in design.md: a fixed order (light phase first, short stages first in each phase); fail-fast only in `check` and `full-check`; no release.

## Release

SemVer: no release — test tooling only; no behavior change.
Target version: none (no release).
