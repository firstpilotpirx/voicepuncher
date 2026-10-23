## Why

In the last day `just test` ran 174 times (7 h, median about 2 min), about half of them failed, many of them the intentional Red step of TDD. The Red step only has to show that the one new test fails for the expected reason; running the whole unit, integration and architecture suite for it is wasted time. `AGENTS.md` says "`just test` after every edit" and the TDD section says Red "write the test … and run it", so sessions run everything. The user decided: in TDD the Red step runs only the one test that is meant to fail, nothing else.

## What Changes

- The Red step runs only the new test (one nextest filter or one Gherkin scenario), never the whole suite.
- A `just` recipe runs one named test (or scenario) quickly, with the same nextest config and time limits.
- `AGENTS.md` § TDD and § How to verify a change, and the agent-kit `tdd` skill, say so.

Current stage: PoC. Target stage: PoC.
Scope: which tests the TDD steps run, and a recipe to run one test.
Acceptance bar: a TDD task's Red run executes only its new test; the stage journal shows Red runs of seconds, not minutes; `just test` runs drop accordingly.
Out of scope: `just check` / `full-check` and their stages.

## Capabilities

### Modified Capabilities

- `change-workflow`: the TDD Red step runs only the new test.

## Impact

- `AGENTS.md`, `justfile` (a one-test recipe, e.g. `just test-one <filter>`), agent-kit `tdd` skill and its rule.

## Open points

- The Green step: proposed — run the same one test until green, then the tests of the touched package (`test@<member>`) once; the full `just test` once per task group, not after every edit.
- Whether a one-test run goes into the stage journal: proposed yes, as recipe `test-one`, never counted as the change's passing check.
- The agent-kit `tdd` skill lives in agent-kit: proposed to file that part there when this change runs.

## Release

SemVer: no release — workflow and tooling only.
Target version: 0.37.4.
