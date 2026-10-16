# Proposal

## Why

On 2026-10-16, during the work on `shrink-check-lock`, `just e2e-smoke` with `VOICEPUNCHER_VOICE_SAMPLES=` (empty: no recordings of the user's voice) ran the `@voice_samples` E2E scenarios, and they failed with `no recorded commands in`, although the test binary had printed `GATED: VOICEPUNCHER_VOICE_SAMPLES not set, skipping @voice_samples scenarios`. README § Testing and AGENTS.md § How to verify a change promise that these scenarios run only where the user's recordings exist.

The cause is how cucumber-rs 0.23 filters scenarios. `Cucumber::filter_run` (`cucumber-0.23.0/src/cucumber.rs`, lines 704–743) applies the harness's own filter only when the command line has neither `--name` nor `--tags`. A filter on the command line replaces the harness's filter instead of being combined with it. Both Gherkin harnesses keep their gates in that filter:

- **E2E, broken today.** `e2e-test/tests/cucumber_e2e.rs` drops `@wip` scenarios and, without recordings, `@voice_samples` scenarios in its filter, prints the GATED line, then calls `filter_run_and_exit`. Every E2E recipe passes `--tags` (`_e2e-smoke`, `_e2e-quick`, `_e2e-bench`, `_refine-eval`, `_e2e-resilience`, `_e2e-seams`), so the gate applies in none of them: the GATED line is printed and the scenarios run anyway. The `justfile` exports `VOICEPUNCHER_VOICE_SAMPLES` empty when `~/.voicepuncher/voice-samples` does not exist, so on any machine without the recordings `just e2e-smoke`, and with it `just check` and `just full-check`, fails: the five `@smoke @voice_samples` scenarios of `voice_control.feature` (16 runs: four scenarios and a 12-row outline) fail in the step "the user's own recordings of the commands exist on the machine" (`recordings_exist` in `e2e-test/tests/steps/voice_control/session.rs`). The user's machine has the recordings, so it went unnoticed.
- **Slice and component, latent.** `app/tests/cucumber_integration.rs` drops `@wip` scenarios and, without `VOICEPUNCHER_REAL_KEYBOARD_IT`, `@real_keyboard` scenarios in its filter; any `--tags` or `--name` run bypasses both. `just test` passes no filter, so it is safe. A direct run is not: `cargo test -p voicepuncher --test cucumber_integration -- --name paste` (the form `fix-overlay-hide-test-race` uses to rerun one scenario) or `-- --tags @component` runs "Hotkey press after a paste is registered". That scenario posts synthetic presses of right Option, the app's default hotkey, and pastes test text into whatever app has focus; in an agent sandbox it fails without the Accessibility permission.
- **`@wip`.** The recipes repeat `and not @wip` in their `--tags`, so they are safe today, but a `--name` run executes a matching `@wip` scenario, and `_test-real-keyboard` passes `--tags @real_keyboard` without `and not @wip`.

## What Changes

- **The gates hold under any selection.** In `cucumber_e2e` and `cucumber_integration`, a scenario runs only when it matches the command line's selection (none, `--tags` or `--name`) and passes the harness's gates:
  - a `@wip` scenario never runs;
  - a `@voice_samples` scenario runs only where the user's recordings exist (`VOICEPUNCHER_VOICE_SAMPLES` set and not empty);
  - a `@real_keyboard` scenario runs only with `VOICEPUNCHER_REAL_KEYBOARD_IT` set.
- **The E2E GATED line names the case.** It no longer says "not set" when the variable is set but empty.
- **Test-first.** A reproduction comes first: a test that a `--tags` or `--name` selection leaves out a scenario whose gate is closed fails on today's code and is committed alone, its message saying it is intentionally red (AGENTS.md § TDD). The fix turns it green, and it stays.
- **Nothing else moves:** no app code; no scenario, step, tag or `.feature` text; the recipes keep their names and arguments.

**Open for design** (not chosen here):

- How the selection and the gates combine. Candidates: the harness reads its command line itself (`fail-hung-tests-fast` D4 adds that reading, through `Cucumber::with_cli`) and runs one filter that requires both; a gated scenario skips itself at run time (both harnesses call `fail_on_skipped()`, which fails a skipped step unless the scenario is tagged `@allow.skipped`); the recipes add the gates to their `--tags` (this leaves `--name` and direct `cargo test` runs unguarded, short of the acceptance bar).
- Where the gate decision lives (both harnesses already depend on the E2E harness library, where `fail-hung-tests-fast` puts its argument reading) and at which test level it is proven.
- Whether the suites keep `and not @wip` in their selections once the harness enforces it.

## Capabilities

### New Capabilities

None. A fix of the test harnesses with no app behavior change: `skip_specs: true`.

### Modified Capabilities

None. The voice-control scenarios that need the user's recordings keep their precondition; only the harness that decides where they can run changes.

## Impact

- `e2e-test/tests/cucumber_e2e.rs` and `app/tests/cucumber_integration.rs`: the entry points and their gates.
- Possibly a shared module of the E2E harness library (`e2e-test/src/`), with its unit tests.
- Possibly the suite selections, only if design.md changes them: in the `justfile` recipes today, in `.config/nextest.toml` (`run-extra-args`) once `fail-hung-tests-fast` has landed.
- `README.md` § Testing and `AGENTS.md`: wording only, if needed; both already promise that `@voice_samples` scenarios run only where the recordings exist and that `@wip` scenarios do not run.
- Build inputs change (`app/`, `e2e-test/`, maybe the `justfile`): `just check` after each task group, `just full-check` before the merge. `e2e-test/tests/cucumber_e2e.rs` and the `justfile` are in `recognition_paths`, so that `full-check` runs the 100-recording accuracy sample.
- No manual run: the app is untouched; hotkey, recording, overlay, paste and permissions behave as before.
- In-flight changes: `recognize-users-quiet-restart` adds `@smoke @voice_samples` scenarios, which the gate covers as written; `fix-overlay-hide-test-race` reruns slice scenarios with `--name`, none of them gated, so its runs select what they select today.

## Dependencies

- **Lands after `fail-hung-tests-fast`**, which must be archived on `main` first. Its D4 rewrites these two entry points to read their own command line, so that they answer nextest's custom-harness protocol, and moves the suite selections into `run-extra-args` in `.config/nextest.toml` (for example `["--tags", "@smoke and not @wip"]`). This change builds on that argument reading instead of editing the same lines in parallel. The defect survives `fail-hung-tests-fast`: the selection it hands to cucumber still replaces the gates.

## Scope

- **Lifecycle stage:** current PoC, target PoC. A test-harness fix; the product stage does not move.
- **Scope:** the `@wip`, `@voice_samples` and `@real_keyboard` gates of the two Gherkin harnesses, under every selection: none, `--tags`, `--name`.
- **Acceptance bar:**
  - The reproduction fails on today's code because a gated scenario is selected, and is committed alone as intentionally red before the fix.
  - Without recordings: `VOICEPUNCHER_VOICE_SAMPLES= just e2e-smoke` runs no `@voice_samples` scenario, says once why (the GATED line), and passes; every other `@smoke` scenario still runs, so the scenario count drops by exactly the `@voice_samples` runs (16 today). An unset variable counts as an empty one.
  - With recordings: `just e2e-smoke` on the user's machine runs the same scenarios as before the change, `@voice_samples` included (same count in cucumber's summary), and passes.
  - Under a filter: without `VOICEPUNCHER_REAL_KEYBOARD_IT`, `cargo test -p voicepuncher --test cucumber_integration -- --name paste` and `-- --tags @component` run no `@real_keyboard` scenario; without recordings, a `cucumber_e2e` run selected by `--tags '@smoke and not @wip'` or by a `--name` that matches a `@voice_samples` scenario runs none of them; no run of either harness, whatever its selection, runs a `@wip` scenario. With the gate open, the same selection still runs the gated scenario: with `VOICEPUNCHER_REAL_KEYBOARD_IT` set, `--tags @real_keyboard` (`just test-real-keyboard`) selects its scenario.
  - Without a filter, both harnesses select what they select today: `just test` runs as many slice and component scenarios as before.
  - No bar is loosened: no scenario is dropped, re-tagged, marked `@wip` or retried.
  - `just test` passes; `just check` passes after each task group and `just full-check` on the final commit.
- **Out of scope:**
  - The plain-Rust tests that read the recordings: `app/tests/voice_samples_it.rs` and `voice_forms_it.rs` return no sessions when `read_dir` of the empty path fails, and `command-spotting/tests/engine_it.rs` returns no clip when it cannot open the relative `session-2.wav`. They skip correctly today, only without a GATED line for an empty variable.
  - A variable that names a directory without the recordings the scenarios read: those scenarios run and fail on the missing files, as today; a wrong path is an error to see, not a reason to skip.
  - Changing cucumber-rs so that it combines the two filters itself: an upstream change.
  - Which scenarios carry `@wip`, `@voice_samples` or `@real_keyboard`, and their steps.
  - The time limits and the custom-harness protocol (`fail-hung-tests-fast`).
  - App code, the services and their protocols.

## Release

No release: no app behavior changes; `version` in `app/Cargo.toml` keeps its value.
