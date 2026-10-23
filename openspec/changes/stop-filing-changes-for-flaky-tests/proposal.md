## Why

A failed test fails the run and, by the rule in `AGENTS.md` § How to verify a change ("A flake is a failure"), the change session files a fix with `/create-change` and declares it in `depends_on`, so the branch waits for the fix. In the last day this blew up the queue: `full-check` passed 11 of 27 runs, almost every failure was an E2E test that flakes under machine load (voice control, start cues, service seams) outside the change's scope, and each one spawned a new change and blocked the original — `refresh-human-docs` (#293) waited 17 h, `fix-load-sensitive-confirm-tests` (#297) 14 h, `fix-dictation-smoke-and-recording-test-under-load` (#301) 7.5 h after its green check. With the launch cap filled by such blocked changes the orchestrator deadlocked overnight.

The user decided: a flaky test never creates a change, never blocks a change and is not rerun. A test known to flake is frozen — marked and not run at all — so it does not get in the way of other sessions; the user fixes frozen tests separately, from time to time.

## What Changes

- A session that hits a failure in a test outside its scope marks the test flaky and freezes it; it files no change, declares no `depends_on`, does not rerun the test and goes on to land.
- A frozen test does not run in `just test`, `check`, `full-check` or any E2E recipe, in any session, until it is unfrozen.
- The frozen tests are listed in one place (test, when and where it flaked, what ran beside it), and a recipe lists them and runs them on demand so the user can fix them.
- Unfreezing a test is part of fixing it.
- `AGENTS.md` § How to verify a change and the agent-kit rules that tell a session to file found work (`openspec.md`, `proceed-change`) say so.
- A failure in a test the change touched stays a failure of the change, fixed in the change.

Current stage: PoC. Target stage: PoC.
Scope: the flake rule, freezing known flaky tests, the frozen list and its recipe.
Acceptance bar: a session whose check fails only on an out-of-scope test freezes it, lands without filing a change and without a rerun; later runs in every session skip the frozen test; the recipe lists and runs the frozen tests.
Out of scope: fixing the flaky tests themselves.

## Capabilities

### Modified Capabilities

- `change-workflow`: out-of-scope flaky tests are frozen, never filed as a change, never rerun, never block.

## Impact

- `AGENTS.md`; `justfile` (stage filters, a recipe such as `just frozen-tests`); nextest config (`.config/nextest.toml`) and the cucumber harnesses (a tag such as `@frozen` for scenarios); agent-kit `~/.claude/rules-custom/openspec.md` and the `proceed-change` skill.

## Open points

- How a test is frozen: proposed one list file on `main` (e.g. `.config/frozen-tests.toml`) read by nextest filters and the cucumber harnesses, rather than editing each test, so a session freezes a test with one line.
- How the freeze reaches other sessions at once: proposed the freezing session pushes the list line straight to `main` (like a filed change), and every run reads the list from `main`.
- How "out of scope" is decided: proposed — the test's files and the code it exercises are untouched by the branch.
- Whether frozen tests lower the bar too far: proposed `full-check` prints the frozen count, and a test frozen for more than N days is reported.
- The agent-kit part is a change in agent-kit's own repository: proposed to file it there when this one runs.

## Release

SemVer: no release — workflow and test tooling only.
Target version: 0.37.4.
