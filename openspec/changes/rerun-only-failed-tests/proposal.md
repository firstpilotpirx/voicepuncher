## Why

When a check fails on one test, the agent fixes it and reruns the whole stage, often the whole check, to see the fix work. Research of 2026-10-18 (report https://claude.ai/artifact/HaDTCo6LEo833xZijMqCh7) found sessions running `just e2e-smoke` (≈ 5 min, 178 scenarios plus 5 test binaries) several times in a row while debugging one failing test, and one session running `just full-check` 4 times in 10 minutes for one failing test. Over 140 recorded runs, 294 of 954 minutes went to runs that failed. The user agreed (2026-10-18): after a failure, rerun only what failed first.

## What Changes

- Each check and E2E recipe can rerun only the tests and scenarios that failed in its last run (or ones named by the agent), with the same services, ports and time limits as the full stage.
- After a failure, the agent first reruns only the failed tests to check the fix; once they pass, the stage runs whole one time, so a fix that breaks another test is still caught before merge.
- A failed run records which tests failed, so the rerun needs no copying of names by hand.

Current stage: PoC. Target stage: PoC.
Scope: `justfile` recipes (`test`, `e2e-smoke`, `e2e-quick`, `e2e-seams`, `e2e-resilience`, `refine-eval`) taking a test or scenario filter; a record of the last run's failures; AGENTS.md § How to verify a change (the rerun rule).
Acceptance bar: after a run where one `*_it` test and one Gherkin scenario failed, one command reruns exactly those two under their stage's services and limits; the full stage then runs once and passes; the rule is in AGENTS.md.
Out of scope: fixing flaky tests (`fix-flaky-tests`); skipping stages that passed (`reuse-passed-check-stages`); counting a rerun of only the failed tests as the change's passing check.

## Capabilities

### Modified Capabilities

- none: test tooling only; no product capability changes.

## Impact

- `justfile`, `.config/nextest.toml` (nextest filters and cucumber `--name` selections), `e2e-test/tests/cucumber_e2e.rs` (if a scenario list must be passed in), AGENTS.md.
- Possibly agent-kit `kit run` / `kit verify`, which already parse failed tests and their locations from a run's output (`~/.claude/rules-custom/kit-placement.md`).

## Open points

- Where the last run's failures are kept: proposed — a file per worktree written by the recipe (e.g. `target/last-failures.txt`).
- How Gherkin scenarios are selected: proposed — cucumber `--name` with the scenario names of the failed run (the harness already keeps gates under `--tags` and `--name` selections).
- Whether the generic part (parse failures, rerun them) belongs to agent-kit: proposed — the rule goes to agent-kit if reusable, the recipes stay here.
- Target version: proposed no release — tooling only.

## Release

SemVer: no release — test tooling only; no behavior change.
Target version: 0.18.0.
