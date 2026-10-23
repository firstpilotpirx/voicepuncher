## 1. Harnesses skip frozen scenarios

- [ ] 1.1 Red: unit tests in `e2e-test/src/scenario_filter/tests.rs` — a frozen scenario does not run, its feature's other scenarios do, the `FROZEN:` line names it; run them, see them fail
- [ ] 1.2 Green: `scenario_filter` reads `VOICEPUNCHER_FROZEN_SCENARIOS` and excludes the listed names; `cucumber_e2e.rs` and `cucumber_integration.rs` apply it and print the `FROZEN:` lines
- [ ] 1.3 `just test`; commit

## 2. The frozen list and the effective list

- [ ] 2.1 Red: `_test-frozen-tests` self-test — effective list = (worktree ∪ `origin/main`) − entries the branch removed; the filterset and scenario lines it exports; the count and the >7-day names; run it, see it fail
- [ ] 2.2 Green: `.config/frozen-tests.toml` (empty list, format comment) and its `stage_inputs` line; `_frozen-env` recipe; `_stages` fetches `origin main`, exports `VOICEPUNCHER_FROZEN_FILTER` and `VOICEPUNCHER_FROZEN_SCENARIOS` unless set, prints the count and stale entries
- [ ] 2.3 Both variables volatile in `_stage-keys`; `_test-stage-keys` asserts they change no key
- [ ] 2.4 `just test`; commit

## 3. Every nextest selection excludes frozen tests

- [ ] 3.1 Red: `_test-frozen-tests` asserts `_smoke-part-filter` and `_it-filter` AND `not (<frozen>)` and `_test`'s selection does too; run it, see it fail
- [ ] 3.2 Green: `_smoke-part-filter`, `_it-filter` and `_test` (normal and rerun selection) add the frozen exclusion
- [ ] 3.3 `just test`; commit

## 4. Freezing, listing and running frozen tests

- [ ] 4.1 Red: `_test-frozen-tests` — `freeze-test` on a scratch repository with a bare `origin` commits the entry on the branch and pushes it to `main`; `frozen-tests` lists both entries with their fields; `frozen-tests run` plans each entry's stage with its selection; run it, see it fail
- [ ] 4.2 Green: `freeze-test`, `frozen-tests` and `frozen-tests run` recipes
- [ ] 4.3 `just test`; commit

## 5. The rule

- [ ] 5.1 `AGENTS.md` § How to verify a change: the flake rule becomes "freeze with `just freeze-test`, no `/create-change`, no `depends_on`, no rerun; a touched test is fixed in the change; unfreezing is part of a fix"; recipe list and README § Testing mention `frozen-tests`
- [ ] 5.2 File the agent-kit change (`proceed-change` stage 10 and `openspec.md` stop asking for a fix change for an out-of-scope flake) with `create-change` in the agent-kit repository
- [ ] 5.3 `just test`; `just check` once; commit

## 6. Version and manual checks

- [ ] 6.1 No version bump (no release: workflow and test tooling only)
- [ ] 6.2 Manual checks: none
