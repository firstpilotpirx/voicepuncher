## Why

A failed test fails the run and, by the rule in `AGENTS.md` § How to verify a change ("A flake is a failure"), the change session files a fix with `/create-change` and declares it in `depends_on`, so the branch waits for the fix. In the last day this blew up the queue: `full-check` passed 11 of 27 runs, almost every failure was an E2E test that flakes under machine load (voice control, start cues, service seams) outside the change's scope, and each one spawned a new change and blocked the original — `refresh-human-docs` (#293) waited 17 h, `fix-load-sensitive-confirm-tests` (#297) 14 h, `fix-dictation-smoke-and-recording-test-under-load` (#301) 7.5 h after its green check. With the launch cap filled by such blocked changes the orchestrator deadlocked overnight. The user decided: a flaky test never creates a change and never blocks a change; flaky tests are fixed separately.

## What Changes

- A change session that hits a failure in a test outside its scope files no change and declares no `depends_on` for it.
- It records the flake (test, recipe, branch, commit, what ran beside it) in one place the user fixes separately, and the change goes on to land.
- `AGENTS.md` § How to verify a change and the agent-kit rules that tell a session to file found work (`openspec.md`, `proceed-change`) say so.
- A failure in a test the change touched stays a failure of the change, fixed in the change.

Current stage: PoC. Target stage: PoC.
Scope: the flake rule and how a session proceeds after an out-of-scope test failure.
Acceptance bar: a session whose `full-check` fails only on an out-of-scope flaky test records it and lands without filing a change; the flake list holds it.
Out of scope: fixing the flaky tests themselves.

## Capabilities

### Modified Capabilities

- `change-workflow`: an out-of-scope flaky test is recorded, never filed as a change, never blocks.

## Impact

- `AGENTS.md`; agent-kit `~/.claude/rules-custom/openspec.md`, the `proceed-change` skill; possibly `kit land` and the `justfile` (`rerun-failed`).

## Open points

- How the change lands past the flake: proposed — rerun only the failed stage once (`just rerun-failed`); it lands when that passes, and a second failure of the same out-of-scope test lets it land too, recorded as a flake. This relaxes "never rerun unchanged" for out-of-scope tests only.
- Where flakes are recorded: proposed one list file on `main` (e.g. `docs/flaky-tests.md`) or the stage journal plus `just stage-times` showing flaky tests; proposed the stage journal, no file to merge.
- How "out of scope" is decided: proposed — the test's files and the code it exercises are untouched by the branch.
- The agent-kit part is a change in agent-kit's own repository: proposed to file it there when this one runs.

## Release

SemVer: no release — workflow rules only.
Target version: 0.37.4.
