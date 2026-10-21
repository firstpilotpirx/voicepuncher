# Design

## Decisions

### 1. The failures come from the stage journal

`_stages` already appends every run of `test`, `check`, `full-check` and the E2E recipes to the stage journal, with each stage's result and `failed_tests`: nextest's `<package>::<binary> <test>` names and `scenario: <name>` for each Gherkin scenario with a failed step. A run stops at its first failed stage, so a failed run has one failed stage. `just rerun-failed` reads the journal's last record of this worktree: when it failed, it takes that stage and its failed tests; when it passed, it prints that there is nothing to rerun and exits 0. A record that ended before its first stage (a lock wait given up or interrupted, a failed setup) tested nothing and is passed over for the one before it. No new file: the record is already written, survives `cargo clean` and holds the stage the tests belong to.

*Alternatives:* `target/last-failures.txt` written by each recipe (a second record of the same facts, and the stage would be missing).

### 2. One stage reruns through `_stages`, with a selection in two variables

`rerun-failed` runs `_stages <failed stage>` with `VOICEPUNCHER_STAGES_RECIPE=rerun-failed`, so the rerun takes the machine lock, the services on the test ports, the build and the time limits of the nextest profile exactly as the stage did, and is journaled under its own recipe name. The selection travels in two variables that the stage recipes and the Gherkin harnesses read:

- `VOICEPUNCHER_RERUN_FILTER`: a nextest filterset, `binary_id(=<binary>) & test(=<test>)` per failed test, joined with `|`, plus the harness binary when scenarios are named. `_test` replaces its selection with it; every nextest call of an E2E recipe adds it with `-E` and `--no-tests=pass`, since one call's binary may hold none of the failed tests.
- `VOICEPUNCHER_RERUN_SCENARIOS`: the failed scenario names, one per line. Both harnesses (`cucumber_integration`, `cucumber_e2e`) select exactly those names instead of the tags of their profile; the gates (`@wip`, `@voice_samples`, `@real_keyboard`) still hold. This goes through `scenario_filter`, which already owns the harnesses' selection.

A failed stage with no named test (fmt, lint, build, or a harness that hung without a failed step) reruns whole, which is the same command without the variables.

*Alternatives:* cucumber `--name` through `run-extra-args` (nextest has no per-run way to add harness arguments; the profile's are fixed); a new recipe per stage (eight copies).

### 3. Names given by hand, and proof that each one ran

`just rerun-failed <name>...` takes names in the journal's form instead of the record's (the stage still comes from the record: its failed stage, or after a passed run of one stage that stage). With `NEXTEST_SUCCESS_OUTPUT=immediate` the rerun's output shows every test and scenario that ran; `rerun-failed` copies it and fails, naming each one, when a name never appears as a passed or failed test or as a run scenario. So a mistyped or renamed test cannot pass by running nothing.

### 4. The rule lives in AGENTS.md; the kit is unchanged

The rerun depends on this project's recipes, journal and harnesses, so the recipe stays here. AGENTS.md § How to verify a change gains the rule: after a failure, fix, then `just rerun-failed` until it passes, then the failed stage's recipe (or the check) once whole; a rerun never counts as the change's passing check. `kit land` accepts only a check that `kit verify` recorded on the commit, so a `rerun-failed` run counts for nothing there. A reusable rule for agent-kit is not filed: the kit's verify already reruns whole checks by design.

## Test levels

No spec scenario (`skip_specs: true`): no product behavior changes. The scenario selection is Rust and gets unit tests; the recipe is checked by recipe runs with `VOICEPUNCHER_STAGE_JOURNAL` at a scratch file and scratch failing tests that are never committed.

| Acceptance item | Level |
|---|---|
| rerun names select exactly those scenarios, gates still hold | unit (`scenario_filter`) |
| one `*_it` test and one slice scenario fail in `just test`; `just rerun-failed` reruns exactly those two | recipe run |
| a fixed test reruns green, then `just test` passes whole | recipe run |
| a name that never ran fails the rerun | recipe run |
| an E2E stage reruns one scenario under its services and lock | recipe run (`e2e-regression`, scratch failing step) |
| the last run passed: nothing to rerun | recipe run |

No manual run: hotkey, recording, overlay, paste and permissions are untouched.

## Dependencies

Declared at filing: `speed-up-tests` reworks the E2E recipes and the suite profiles this change adds the selection to; this change stacks on its branch and lands after it.
