# Design

## Context

See proposal.md § Why. The stage runner (`dev-tools`) runs, keys, journals and tables every stage; benches run under the machine lock only; the journal line's 15 fields are a kept contract.

## Goals / Non-Goals

**Goals:** bounded model stages; a cheaper smoke set and `test` without lost coverage (what leaves `@smoke` stays in `@regression`); benches and area runs in the journal.

**Non-Goals:** parallel model stages (timing scenarios need the machine alone); service warm-up changes.

## Decisions

- **Budgets in `.config/stages.toml` (`stage_budgets`: `<stage> <seconds>`), enforced by the runner, not nextest's slow-timeout**, which would kill the harness as one test and name nothing. A model stage over budget fails the run; `test` over budget prints `● test: 72 s past its budget of 60 s`. Values: measured after the speed-up plus 20 %, set in the last task group.
- **Scenario times come from the harness:** the E2E harness's `after` hook prints `scenario time: <secs> <name>` and ends with its five slowest; the runner reads them from the run's log and adds the three busiest processes (`ps`).
- **Fast pacing feeds the silence between a session's steps ten times faster (a 100 ms frame every 10 ms); speech is fed at once as now.** Instant silence would collapse the capture-time map the app keeps for the lead-in. `@cues` keeps real time. A scenario failing at the fast pace stays in real time and is named here: none expected.
- **Smoke per rule by tags only:** the first scenario of each rule of `local_services` and `speech_recognition` keeps `@smoke`, the others get `@regression`; a dev-tools scenario counts the tags per rule from the feature files.
- **Benches are stages:** `bench` and `streaming-bench` run `_stages bench` / `_stages streaming-bench` with their arguments in `VOICEPUNCHER_BENCH_ARGS`; model stages (the e2e lock) that start no service, with no key, pass or reuse. The journal line gets a 16th field `args` (null elsewhere); the per-branch row a bench column.
- **The needed-tests map is computed:** areas are the workspace members; a row holds the member's `test` part, the stages its paths feed (`stage_inputs`) and the bench options of a `bench_areas` table (`<glob> <recipe and options>`). `just test-area <path>` runs the `test` stage as recipe `test-area` with only the owning member's part (`VOICEPUNCHER_TEST_PARTS`), never a pass. A unit test over the real tables fails on a glob matching nothing.
- **The task-group warning reads `tasks.md` of the branch's change** (`<type>/<name>`; groups are `##` headings, open means an unchecked task) for `test`, `check` and `full-check`.
- **The secret scanners' range comes from `dev-tools scan-range`:** `origin/main..HEAD` on a branch, `--all` on `main`, on a detached HEAD and on a branch whose repository has no `origin/main` (a scan with no base scans everything) (trufflehog: `--since-commit`).
- **The per-rule harness helper moves to `test-steps`** (`rule_tests`, `read_rule_harness_call`, `rule_command_line`), used by the app and the workflow harnesses; the E2E harness keeps its one test.

## Risks / Trade-offs

- [Fast pacing shifts the lead-in's timing] → ten times, not instant; a failing scenario goes back to real time.
- [A budget fails on a loaded machine] → the failure's busiest processes tell a loaded run from a grown scenario.

## Test levels

Component = `dev-tools/tests/features/change_workflow.feature` with scratch stand-ins.

| Scenario | Level |
|---|---|
| A stage within its budget passes | component |
| A stage past its budget fails naming the slowest scenarios | component |
| The unit stage past its budget warns and passes | component |
| A hands-free session not under timing ends before its audio would | E2E `.feature` (`@regression`) |
| A rule with several scenarios runs one in the check | component (over the feature files) |
| A streaming bench run is journaled with its arguments | component |
| A bench run is never a pass | component |
| The summary shows check and bench time per branch apart | component |
| The map names the tests of an area | component |
| A run over one area runs only its tests | component |
| An area that exists nowhere fails the unit run | component + unit (the real tables) |
| A full suite on an open task group that is not the last warns | component |
| A full suite on the last task group runs without a warning | component |
| A branch scans the commits since main | component |
| Main scans the whole history | component |
| A failed rule is reported by its name | component + unit (`test-steps`) |
| A rerun covers only the failed rule's scenarios | component |
| An overlay change skips the seams, outages and start cues | component (as now) |
| A change to the chunking code runs the seams stage | component (as now) |
| A file no stage names runs every stage | component (as now) |
| A change to the stage runner's tests reuses every model stage | component |
