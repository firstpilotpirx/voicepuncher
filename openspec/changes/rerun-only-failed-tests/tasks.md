# Tasks

No spec scenario (`skip_specs: true`). Recipe behavior is checked by recipe runs with `VOICEPUNCHER_STAGE_JOURNAL` at a scratch file; scratch failing tests are never committed (design.md § Test levels). No version bump: test tooling only.

## 1. Scenario selection by name (decision 2)

- [ ] 1.1 Red: unit tests in `e2e-test/src/scenario_filter.rs` — rerun names select exactly the named scenarios over the tags, an unnamed scenario does not run, a closed gate still holds back a named one; run them and see them fail.
- [ ] 1.2 Green: `scenario_filter` reads the rerun names (`VOICEPUNCHER_RERUN_SCENARIOS`, one per line) and selects them instead of `--name`/`--tags`; `cucumber_integration.rs` and `cucumber_e2e.rs` pass the variable in.
- [ ] 1.3 `just test`; commit.

## 2. The recipes (decisions 1–3)

- [ ] 2.1 Red: `just rerun-failed` does not exist.
- [ ] 2.2 Green: `_test` and every nextest call of the E2E recipes take `VOICEPUNCHER_RERUN_FILTER` (`-E`, `--no-tests=pass` in the E2E calls).
- [ ] 2.3 Green: `rerun-failed *names` reads the worktree's last journal record, builds the two variables, runs `_stages <failed stage>` as recipe `rerun-failed` with `NEXTEST_SUCCESS_OUTPUT=immediate`, and fails naming every name that never ran; a passed last run prints that nothing is to rerun; a failed stage with no named test reruns whole.
- [ ] 2.4 Check: a scratch failing `*_it` test and a scratch failing slice scenario fail `just test`; `just rerun-failed` runs exactly those two and fails; with both fixed it passes, then `just test` passes whole; a mistyped name fails the rerun; a scratch failing `@regression` step reruns alone under `e2e-regression`'s services and lock; commit.

## 3. Docs and the full check

- [ ] 3.1 `AGENTS.md` § How to verify a change: the rerun rule (failed only first, then the stage whole once; a rerun is never the passing check); `README.md` § Testing and the `justfile` comments of the changed recipes.
- [ ] 3.2 Version: no bump — test tooling only (proposal § Release); `app/Cargo.toml` stays at 0.26.0. Manual checks: none.
