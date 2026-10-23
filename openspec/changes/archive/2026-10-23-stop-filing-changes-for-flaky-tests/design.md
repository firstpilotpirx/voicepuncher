## Context

See proposal.md — Why. Plain tests are nextest tests picked by `-E` filtersets; scenarios run inside the two cucumber harnesses, whose `scenario_filter` already narrows a rerun. Test names have one form, the stage journal's: `<package>::<binary> <test>` and `scenario: <name>`.

## Goals / Non-Goals

**Goals:** one line freezes a test everywhere at once; a frozen test runs in no recipe; the fixing branch runs it again.
**Non-Goals:** fixing any flaky test; guarding automatically whether a test is in the change's scope (the session judges it from the branch's diff).

## Decisions

**D1 — One list file, `.config/frozen-tests.toml`.** One `[[frozen]]` table per test with `test` (the journal's form), `frozen` (the day), `recipe`, `stage`, `beside` (what ran beside it: other sessions' runs, the lock holder) and `branch`. Parsed by the justfile's perl, so no build precedes a run. Rejected: a tag on each test (no list); nextest `default-filter` (no metadata; the `ci` profile overrides it).

**D2 — The effective list merges `main`.** `_stages` fetches `origin main` and computes: (this worktree's list ∪ `origin/main`'s list) − the entries this branch removed since its merge base. So a freeze pushed to `main` holds in every worktree's next run, and a fixing branch that deletes the entry runs the test although `main` still lists it.

**D3 — Freezing is one recipe that also pushes to `main`.** `just freeze-test '<name>' <recipe> <stage> '<beside>'` adds the entry, commits it on the branch and pushes the same one-file edit straight to `main` from a temporary worktree of `origin/main`, retrying on a moved `main`. Rejected: landing the line only with the branch (others keep failing for hours).

**D4 — How the list reaches the runs.** `_stages` exports `VOICEPUNCHER_FROZEN_FILTER` (a nextest filterset of the frozen plain tests) and `VOICEPUNCHER_FROZEN_SCENARIOS` (one scenario name per line) unless the caller set them. The three filterset builders AND `not (<frozen>)` into their expression. Both harnesses add a `frozen` exclusion to `ScenarioFilter`: a listed scenario does not run and the harness prints one `FROZEN: skipping scenario '<name>'` line per skipped scenario; a Scenario Outline is frozen as a whole. Both variables are volatile for the stage keys: adding a freeze only shrinks what a stage runs, so a pass stays valid and the failed stage alone reruns after a freeze.

**D5 — Listing and running.** `just frozen-tests` prints each entry; `just frozen-tests run` groups the entries by stage and runs each stage through `_stages` with the rerun selection of its entries and both frozen variables empty, never recorded as a pass. Every `_stages` run prints `● frozen: N test(s)` when N > 0 and names each entry older than 7 days.

**D6 — The rule.** `AGENTS.md` § How to verify a change replaces "file its fix with `/create-change` … declare that change in `depends_on`" with: a failure in a test the branch did not touch is frozen with `just freeze-test`, then the check runs again (passed stages are reused); a failure in a touched test is fixed in the change; unfreezing is part of a fix. The agent-kit rules that ask for a fix change (`proceed-change` stage 10) are agent-kit's own repository: a change is filed there with `create-change`.

## Risks / Trade-offs

- [A real regression frozen as a flake] → only an untouched test may be frozen; every run prints the count and names entries older than 7 days.
- [Two sessions freeze at once] → the push retries on a fresh `origin/main`; entries are independent tables.
- [The fetch fails or costs time] → one quiet fetch per run; a failure keeps the last fetched `origin/main`.

## Test levels

| Scenario | Level |
|---|---|
| An out-of-scope failure is frozen and the change lands | justfile self-test `_test-frozen-tests` (freeze recipe on a scratch repository with a bare `origin`) |
| A failure in a test the change touched is not frozen away | manual run (session rule in `AGENTS.md`; no automation decides scope) |
| A frozen scenario is skipped and named | unit (`scenario_filter` tests) |
| A frozen plain test does not run | justfile self-test (filterset builders AND the frozen exclusion) |
| A freeze on main reaches another worktree | justfile self-test (effective list) |
| The fixing branch unfreezes the test | justfile self-test (effective list) |
| The user lists the frozen tests | justfile self-test |
| The user runs only the frozen tests | justfile self-test (the run plan: stage and selection per entry) |
| A check names tests frozen too long | justfile self-test |
