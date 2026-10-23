## Why

E2E tests that are sensitive to machine load fail when other work runs beside them, and those flakes dominate the last day: `full-check` passed 11 of 27 runs, almost every failure an E2E scenario under load (voice control, start cues, service seams, placed commands). Today only `e2e-cues` has the machine to itself (exclusive lock plus the quiet lock). The light class (`e2e-smoke-voice`, `e2e-regression`, `e2e-resilience`) runs beside another run's heavy stages, and the builds, `just test`, `just release` and `cargo watch` of other sessions run beside any E2E stage without a lock. The user decided, as discussed before: E2E tests under load run with a lock — no two at once, and nothing else heavy runs while one does.

## What Changes

- Every E2E model stage takes the machine lock exclusively: no light/heavy overlap, one E2E stage on the machine at a time.
- While an E2E stage runs, the build and test stages of other runs (`fmt`, `lint`, `test`, `metrics`, `build`, `bundle`, `build-e2e`, standalone `just test`) wait, as they already do for `e2e-cues` (quiet lock).
- `AGENTS.md` § How to verify a change (Lock classes) is rewritten to match.

Current stage: PoC. Target stage: PoC.
Scope: the lock classes of the `justfile` and the quiet lock's holders.
Acceptance bar: two sessions' checks never run E2E stages at the same time, and no build or test stage starts while an E2E stage runs (shown by the stage journal's lock times); the load-sensitive scenarios that flaked in the last day pass in a series of runs beside a busy queue.
Out of scope: fixing individual flaky tests; whether a flake files a change (`stop-filing-changes-for-flaky-tests`).

## Capabilities

### Modified Capabilities

- `change-workflow`: E2E stages run alone on the machine.

## Impact

- `justfile` (`light_stages`, `exclusive_stages`, `_quiet-lock`, `_stages`), `AGENTS.md`, the light test ports (may go away), agent-kit machine lock rule if it names the classes.

## Open points

- Queue time: making every E2E stage exclusive brings back lock waits (71 h of queue on 2026-10-21 before the class split): proposed to accept it and measure with `just stage-times` after a day.
- Whether standalone `just test` and `cargo watch` also wait: proposed `just test` waits, `cargo watch` (the user's app) does not.
- Whether non-load-sensitive stages (`e2e-bench`, `refine-eval`, `e2e-bundle`) keep sharing: proposed all E2E/model stages exclusive, simplest rule.

## Release

SemVer: no release — test tooling only.
Target version: 0.37.4.
