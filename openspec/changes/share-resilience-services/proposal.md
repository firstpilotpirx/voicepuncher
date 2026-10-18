## Why

The research of 2026-10-18 (report https://claude.ai/artifact/HaDTCo6LEo833xZijMqCh7) measured `e2e-resilience` at 83 min over 140 recorded runs: about 176 s for 8 scenarios. Every outage scenario starts its own set of services (speech recognition for dictation and for commands, command spotting) on free ports of its own, so it can pause, kill and restart them without disturbing other scenarios; most of the stage's time may be service start-up and model loading, not the outages themselves. The idea is to start one shared set of services for the stage and give only the service a scenario breaks a copy of its own, or restart the shared one between scenarios. Whether the gain is significant is unknown: the user wants this researched first, and implemented only when the effect is worth it.

## What Changes

- Measure where `e2e-resilience` time goes: service start-up and model loading per scenario versus the outage, the wait for recovery and the checks.
- If start-up dominates: scenarios share one set of services and each starts a private copy only of the service it breaks (or the shared service is restored to a known state between scenarios), keeping each scenario independent of the others.
- If the gain is small: record the measurement in this change and close it without code changes.

Current stage: PoC. Target stage: PoC.
Scope: the `@resilience` E2E steps (`e2e-test/tests/steps/`), the `e2e-resilience` recipe in the `justfile`.
Acceptance bar: a measurement of start-up versus outage time per scenario is written down; if implemented, `e2e-resilience` is measurably faster (target: at least 30 % less) with every scenario still passing on repeated runs and still independent of scenario order.
Out of scope: changing which outages are tested or what the voice control promises during an outage; other E2E stages.

## Capabilities

### Modified Capabilities

- none: test tooling only; no product capability changes.

## Impact

- `e2e-test/tests/steps/` (resilience and local-services steps), `justfile` (`e2e-resilience`).

## Open points

- Gain threshold to go ahead: proposed — implement only if start-up is more than about a third of the stage's time.
- How a scenario breaks a service without harming others: proposed — a private copy of only the broken service; the alternative is restarting the shared one after each scenario.
- Whether scenarios can then run in parallel: proposed — measure, decide in design.
- Order with `fix-flaky-tests` and `speed-up-tests`: proposed — after them or alongside, since shared services can bring back races; measure with `log-stage-times` if it lands first.
- Target version: proposed no release — tooling only.

## Release

SemVer: no release — test tooling only; no behavior change.
Target version: 0.18.0.
