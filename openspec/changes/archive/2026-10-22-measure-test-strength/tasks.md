# Tasks

## 1. Tools and toolchains

- [ ] 1.1 Pin `llvm_cov_version`, `mutants_version` and `nightly_date` in the `justfile`; add `strength-tools` (installs the two crates, `llvm-tools` on `1.88`, the nightly with `miri`, `rust-src`, `llvm-tools`) and `_require-strength-tools` (names `just strength-tools`, installs nothing); verify `just _require-strength-tools` passes here and fails naming the recipe with a wrong pinned version (`just mutants_version=9.9.9 _require-strength-tools`)
- [ ] 1.2 Add `[profile.mutants]` (inherits `test`, `debug = false`) to the root `Cargo.toml` and `[profile.miri]` (slow-timeout 60 s × 10) to `.config/nextest.toml`; verify `just test` still passes and `cargo nextest run --profile miri --list -p voicepuncher-copy-pool` accepts the profile
- [ ] 1.3 `just test`; commit

## 2. Floors baseline (TDD)

- [ ] 2.1 Red: unit tests in `analyzers/src/floors.rs` — a value under its floor is a breach naming the key, floor and value; a value at or above passes; a key without a floor passes with a note; `write` stores value − 0.5 (one decimal, never below 0) and the commit; the tests fail because the module is missing
- [ ] 2.2 Green: implement `floors.rs` (load/compare/write of `.config/analyzers/strength.toml`); verify `cargo nextest run -p voicepuncher-analyzers` passes
- [ ] 2.3 `just test`; commit

## 3. Coverage analyzer (TDD)

- [ ] 3.1 Red: unit tests in `analyzers/src/coverage.rs` on a saved llvm-cov JSON fixture (`analyzers/tests/fixtures/coverage.json`): per-crate sums of lines and regions, files under `tests/` left out, the report rows and the `strength coverage …` journal rows; the tests fail because the module is missing
- [ ] 3.2 Green: implement `coverage.rs` (runs `cargo llvm-cov nextest` into `VOICEPUNCHER_STRENGTH_DIR`, parses, prints, writes the journal rows to `VOICEPUNCHER_STAGES_REPORT` when set, ratchets with `floors`) and wire `coverage` into `main.rs`; verify the unit tests pass and `voicepuncher-analyzers coverage` prints one row per crate here
- [ ] 3.3 Add the `coverage` recipe and `_coverage` (unsets the empty `VOICEPUNCHER_*` model variables as `_test` does, requires the strength tools); verify `just coverage` passes with no floor file and prints the rows
- [ ] 3.4 `just test`; commit

## 4. Mutants analyzer (TDD)

- [ ] 4.1 Red: unit tests in `analyzers/src/mutants.rs` on saved fixtures (`outcomes.json`, `missed.txt`): tried, caught (caught + timeout), survived and score per crate; the survivor list; the `cargo mutants` command built with `--in-diff` for a diff crate and without for a whole one, `--profile mutants`, `-- --lib` from `test_args`; a crate with no mutant in the diff passes with its note; the journal rows; the tests fail because the module is missing
- [ ] 4.2 Green: implement `mutants.rs` and the `[[mutants.crates]]` config in `.config/analyzers.toml` (copy-pool whole; app-core `app/src/core/**/*.rs` with `-- --lib`), `--diff <patch>`, `--whole`, `--report <dir>`, `--write-baseline`; wire `mutants` into `main.rs`; verify the unit tests pass
- [ ] 4.3 Add the recipes `mutants <path>` (one path, `--report`, no floors) and `_mutants` (writes `git diff --merge-base origin/main HEAD -- app/src/core` to a patch file, runs the analyzer with `--diff`), and `strength-baseline` (`coverage --write-baseline`, `mutants --whole --write-baseline` under `taskpolicy -c utility`); verify `just mutants copy-pool/src/pool/state.rs` prints the table and survivors and `just _mutants` passes on this branch (no core lines changed → copy-pool whole, app-core "no mutants in the changed lines")
- [ ] 4.4 `just test`; commit

## 5. Stages and journal

- [ ] 5.1 Red: extend the `justfile` self-tests — `_test-lock-phases` expects `full-check` to end with `coverage mutants` as trailing unlocked stages, and a `_stage-journal-append` self-test feeds a report with `strength` rows and expects the `strength` field in the record; verify `just test` fails on them
- [ ] 5.2 Green: `trailing_stages := "coverage mutants"` in `_stages` (run after the lock phases, unlocked, the `to_run` early exit kept for the model stages only), `full-check` lists them last, `_stage-journal-append` collects the `strength` rows; verify `just test` passes
- [ ] 5.3 `just test`; commit
- [ ] 5.4 Run `just strength-baseline` in the background (coverage, copy-pool whole, the whole app core: hours) and note its wall time and the scores in this file; verify `.config/analyzers/strength.toml` holds every floor with the commit

## 6. Sanitize recipe

- [ ] 6.1 Add `sanitize` (miri over `core::` without `pipeline::` under `MIRIFLAGS=-Zmiri-disable-isolation` and `--profile miri`; ASan over `main_queue`, `textpasting_macos_it`, `focus_macos_it` with `CARGO_TARGET_DIR=target/sanitize`), with the comment on what is excluded and why; verify `just sanitize` passes here and note its wall time in this file
- [ ] 6.2 Verify the recipe fails on a finding: a temporary core test that reads uninitialized memory and a temporary out-of-bounds read in a sanitized test each fail `just sanitize` naming the test; remove both before committing and note the two outputs in this file
- [ ] 6.3 `just test`; commit

## 7. Tests against the survivors of core/domain (TDD)

- [ ] 7.1 From the `strength-baseline` run of 5.4, list the survivors under `app/src/core/domain/` and mark the ones a unit test at the function's contract can kill; note the list in this file
- [ ] 7.2 Red/Green per survivor: a unit test that fails on the mutant (`cargo mutants -f <file> --test-tool nextest -- --lib` reports it caught) and passes on the code; verify `just test` passes and at least 10 survivors are caught
- [ ] 7.3 Rerun `just strength-baseline` for app-core only is too long: rerun `cargo mutants -f 'app/src/core/domain/**'` in the background and write the new app-core floor by hand from the full-run numbers plus the killed survivors into `strength.toml`; verify `voicepuncher-analyzers mutants --whole` on copy-pool passes against the floors
- [ ] 7.4 `just test`; commit

## 8. Documentation

- [ ] 8.1 AGENTS.md § How to verify a change: the `full-check` row names `coverage` and `mutants` and their time; a **Test strength** paragraph (the floors, `just strength-baseline`, `just mutants <path>`, `just sanitize` and when to run it, `just strength-tools`, the target directory cost); verify `just test` passes (docs feed no stage)
- [ ] 8.2 Check the size budgets: every new file in `analyzers/` under 300 code lines, every function under 40; verify `just lint` and `just test` pass
- [ ] 8.3 Keep `version` at its current value (no release: tooling and tests only); `just test`; commit
