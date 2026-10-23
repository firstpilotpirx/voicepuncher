# Design

## Context

`just rerun-failed` already runs a named set of tests and scenarios inside one stage: it turns names into a nextest filter and a scenario list, runs the stage through `_stages` with `VOICEPUNCHER_RERUN_FILTER` / `VOICEPUNCHER_RERUN_SCENARIOS` (so nothing is reused or recorded as a pass, and the run is journaled under its recipe name), then fails on a name that never ran. Its names come from the stage journal and must be exact (`<package>::<binary> <test>`), which an author of a new test does not have.

## Goals / Non-Goals

**Goals:** a `just test-one <name>...` recipe for the Red and Green steps; `AGENTS.md` and the agent-kit `tdd` skill telling sessions to use it.

**Non-Goals:** E2E scenarios (their harness needs services; `rerun-failed` covers them after a failed check); changing `just test`, `check` or `full-check`.

## Decisions

- **Share the rerun machinery.** The part of `rerun-failed` that turns names into a selection, runs one stage with it and checks every name ran moves into private recipes used by both `rerun-failed` and `test-one`. `test-one` always runs the `test` stage. Alternative — a plain `cargo nextest run <filter>` recipe — rejected: it would bypass the journal, the quiet lock's time limit and the never-ran check, and duplicate the rerun's filter logic.
- **Name forms.** `test-one` accepts the rerun's exact forms plus a bare name: a bare name `x` selects `test(x)` (nextest's substring match on the test name), so `should_reject_empty_chunk` or `chunk::tests::should_reject` both work; `scenario: <name>` selects a slice/component scenario (the `cucumber_integration` harness). A bare name counts as ran when at least one nextest result line contains it.
- **A name that matches nothing fails.** nextest passes with `--no-tests=pass` in a rerun, so the never-ran check is the guard: without it a typo would make a Red step look green.
- **Journal.** `test-one` is journaled under its own recipe name (`VOICEPUNCHER_STAGES_RECIPE=test-one`); like a rerun it neither reuses nor records a pass, so it can never be a change's passing check.
- **Green and the group.** Green and Refactor rerun the same `test-one`; the group ends with one `just test`, whose part reuse already limits it to the packages whose inputs changed. A separate "touched package" step is not added: it would duplicate that reuse.
- **agent-kit.** The `tdd` skill lives in agent-kit; its Red step gets one line: run only the new test, through the project's one-test command named in `AGENTS.md`. It lands as a PR of its own in agent-kit, done as part of this change.

## Risks / Trade-offs

- [A bare name matching several tests runs them all] → acceptable: still seconds, and the output lists them; an exact form narrows it.
- [The workspace build before the run still takes its time] → unchanged; the gain is the test run itself and the skipped self-tests and metrics.

## Test levels

| Scenario | Level |
|---|---|
| A named unit test runs alone | unit — `justfile` self-test of the name-to-selection mapping |
| A named scenario runs alone | unit — `justfile` self-test of the mapping (scenario list, harness selected) |
| A name that matches nothing fails the run | unit — `justfile` self-test of the never-ran check on a captured log |
| A one-test run is journaled but never a pass | unit — `justfile` self-test: the recipe sets its journal name and the rerun marker that blocks reuse and pass records |
| Red runs one test, not the suite | manual run — this change's own Red steps, seen in the stage journal |
| The full suite runs once per task group | manual run — the stage journal of this change |
