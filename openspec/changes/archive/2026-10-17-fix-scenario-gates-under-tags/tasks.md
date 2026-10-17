# Tasks

## 1. Red: reproduce gates bypassed by a selection (design D5, D6)

- [x] 1.1 Verify `fail-hung-tests-fast` is archived on `main` (`openspec list` on `origin/main` no longer lists it, `openspec/changes/archive/` holds it); otherwise stop, this change depends on it
- [x] 1.2 Read, as they are on `main`: the `main` of `e2e-test/tests/cucumber_e2e.rs` and of `app/tests/cucumber_integration.rs`, fail-hung-tests-fast's argument module in `e2e-test/src/`, `e2e-test/Cargo.toml` and the suite selections in `.config/nextest.toml`; note in the stage report where `cli::Opts` is parsed, whether `with_cli` is called, and whether `cucumber` and `regex` are already in `[dependencies]` of `voicepuncher-e2e-test`; verify every case of D5 is settled (options parsed in the argument module, or in the harness, or `with_cli` to add)
- [x] 1.3 Count the `@smoke @voice_samples` runs in `e2e-test/tests/features/` (scenarios plus outline rows; 16 on 2026-10-16) and read cucumber's `N scenarios` line of the last `e2e-smoke` or `check` log of `main` (`kit log --grep 'scenarios'`); verify both numbers are written down for 2.9 and 2.10
- [x] 1.4 Add `e2e-test/src/scenario_filter.rs`, exported from `lib.rs`, with `Gate`, `Selection`, `ScenarioFilter` and `split(opts, gates)` written with today's behavior: `split` returns the options untouched and a filter of the gates alone (today's `has_tag` closures copied), and the line for an empty `VOICEPUNCHER_VOICE_SAMPLES` says "not set"; move `cucumber` (`=0.23.0`) and `regex` (`1`) to `[dependencies]` unless 1.2 found them there; verify `Cargo.lock` is unchanged (`git diff --exit-code Cargo.lock`) and the harness files are untouched
- [x] 1.5 Write the module's `#[cfg(test)] mod tests`: a test World with one step, a feature written to a temp dir (an ungated scenario, a `@wip`, a `@voice_samples` and a `@real_keyboard` scenario, one gate tag on a `Rule` and one on an `Examples` block; no committed `.feature`), options parsed from a command line, `split`, `with_cli`, and the names of the scenarios that ran collected after each run; cover every case of the D6 table (closed gates under `--tags` and under `--name`, `@wip` under `--tags @wip`, `--name` and no selection, open gate, no selection, a selection missing the ungated scenario, the voice-samples gate for unset, empty and a path)
- [x] 1.6 For each case expected to pass on Red (open gate, no selection, selection missing the ungated scenario), break the filter on purpose once and verify the case fails, then restore it (AGENTS.md § TDD)
- [x] 1.7 Fixture runs of the unmodified harnesses: write a temp feature per harness outside the repo (feature tag `@gates_fixture`; an ungated `@allow.skipped` scenario; a `@wip` scenario and one with the harness's own gate tag, not `@allow.skipped`; step texts matching no step definition) and run, without the machine lock and with `VOICEPUNCHER_HANG_PROBE` unset, `kit run cargo test -p voicepuncher --test cucumber_integration -- --input '<dir>/*.feature' --tags @gates_fixture`, then `--name fixture`, and the same for `-p voicepuncher-e2e-test --test cucumber_e2e` with `VOICEPUNCHER_VOICE_SAMPLES=`; verify the `@wip` and gated fixture scenarios are selected and fail; keep an excerpt for the PR body
- [x] 1.8 Run `kit run cargo test -p voicepuncher --test cucumber_integration` without a selection and write down its `N scenarios` line for 2.5
- [x] 1.9 Run `kit run just test` and verify it fails only in the D6 cases marked "fails" (closed gates under `--tags` and `--name`, `@wip` under a selection, the empty voice-samples line), each failure naming the gated scenarios that ran; commit alone as `test: reproduce scenario gates bypassed by --tags and --name` with a message stating it is intentionally red (no `just check`: the group is red by design, D6)

## 2. Green: the selection and the gates in one filter (design D1–D5, § Test levels)

- [x] 2.1 Make `split` destructure `cli::Opts` naming all six fields without `..`, take `re_filter` and `tags_filter` into the `Selection` (name first, then tags, then everything, as `filter_run`) and return the options with both `None`; make `ScenarioFilter::runs` require the selection and every gate over the feature's, rule's and scenario's tags; verify the D6 selection and `@wip` cases pass in `kit run just test`
- [x] 2.2 Add the `GATED: VOICEPUNCHER_VOICE_SAMPLES empty, skipping @voice_samples scenarios` line for an empty value, keeping the unset line; verify the voice-samples gate case passes (closed for unset and empty with different lines, open for a path)
- [x] 2.3 Route `cucumber_e2e` and `cucumber_integration` through the module (D5 four steps: parse options, build gates from the environment, `split`, print the lines on stderr, `.fail_on_skipped().with_cli(opts).filter_run_and_exit("tests/features", move |f, r, s| filter.runs(f, r, s))`); remove both copies of `has_tag`; state in each `main`'s doc comment why cucumber never gets the selection; verify `kit run just test` passes
- [x] 2.4 Add `e2e-test/src/scenario_filter.rs` to `recognition_paths` in the `justfile`; verify `just --evaluate recognition_paths` (or the recipe printing it) lists it
- [x] 2.5 Run `kit run cargo test -p voicepuncher --test cucumber_integration` without a selection; verify its `N scenarios` equals the count from 1.8
- [x] 2.6 Repeat the fixture runs of 1.7 on the modified harnesses: gates closed under `--tags @gates_fixture` and under `--name fixture` exit 0 with `1 scenario (1 skipped)`; gate open (`VOICEPUNCHER_REAL_KEYBOARD_IT=1` with `--tags @real_keyboard`, `VOICEPUNCHER_VOICE_SAMPLES=<any path>` with `--tags @voice_samples`) selects the gated fixture scenario, which fails on its undefined step
- [x] 2.7 Real-feature runs, only after 2.6 passes, without `VOICEPUNCHER_REAL_KEYBOARD_IT`: run `kit run cargo test -p voicepuncher --test cucumber_integration -- --name paste`, then `-- --tags @component`; verify both pass and "Hotkey press after a paste is registered" is not in their output (never run with the variable set, never `just test-real-keyboard`)
- [x] 2.8 Run `VOICEPUNCHER_VOICE_SAMPLES= kit run cargo test -p voicepuncher-e2e-test --test cucumber_e2e -- --name "user's recorded"` without services; verify `0 scenarios` and exit 0
- [x] 2.9 Run `VOICEPUNCHER_VOICE_SAMPLES= kit run just e2e-smoke`; verify it passes, prints one `GATED: VOICEPUNCHER_VOICE_SAMPLES empty, …` line, runs no `@voice_samples` scenario, and its `N scenarios` equals the 1.3 count from `main` minus the `@smoke @voice_samples` runs counted in 1.3
- [x] 2.10 Run `kit run just e2e-smoke` with the recordings of this machine; verify it passes and its `N scenarios` equals the 1.3 count from `main`
- [x] 2.11 Verify no bar moved: `git diff origin/main -- '*.feature'` is empty, no `@allow.skipped`, `@wip` or retry added, no app code (`app/src/`) changed, recipe names and arguments unchanged
- [x] 2.12 Run `kit run just check` and verify it passes; compare its release-build stage time with `main`'s last `check` log and write the difference down for the PR
- [x] 2.13 Commit `fix(test): keep scenario gates under --tags and --name selections`

## 3. Version and manual checks

- [x] 3.1 Verify `version` in `app/Cargo.toml` is unchanged: no bump and no release (proposal § Release)
- [x] 3.2 Manual checks: none (no app code; hotkey, recording, overlay, paste and permissions are untouched)

## Workflow follow-up

- Stage 8: `just full-check` with the 100-recording accuracy sample, since `cucumber_e2e.rs` and the `justfile` are on the recognition path.
- PR body: the Red fixture-run excerpt (1.7), the release-build time difference (2.12), the scenario counts of 2.9 and 2.10, and the checks that ran.
