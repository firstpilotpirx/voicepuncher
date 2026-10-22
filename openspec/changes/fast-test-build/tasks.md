# Tasks

## 1. Baseline measurement

- [ ] 1.1 Measure the build before any move (design D6: touch one `app` file, `cargo build --workspace --tests --timings`, unit-duration sum, wall clock, load average) and the test count of `just test`; record both in proposal.md § Why; commit

## 2. The app's `it` binary and the guard test

- [ ] 2.1 Red: add the guard test of design D4 to `app/tests/architecture.rs` for the `app` member (only harness roots, `it/`, `main_thread/` and helper directories under `tests/`; every `tests/it/*.rs` declared in `it/main.rs`); run it and see it fail naming the 37 test files
- [ ] 2.2 Green: `git mv` every `app/tests/<x>_it.rs` and `<x>_it/` to `app/tests/it/<x>.rs` (and `it/<x>/`), add `app/tests/it/main.rs` with one `mod` per file and `support` by path, point the R4 and R9 expectations at `tests/it/<stem>.rs`, list `it` beside `cucumber_integration` in `app/Cargo.toml`; the guard passes
- [ ] 2.3 Green: build `app/tests/main_thread/main.rs` (design D2) from `visualfeedback_macos_it.rs`, `menubar_macos_it.rs` and `settings_macos_it.rs` as modules with one `main()` listing `<module>::<name>`; `cargo nextest list` shows the same tests
- [ ] 2.4 Rewrite every `binary(<x>_it)` of `.config/nextest.toml` (groups, priorities, limits, the CI default filter) per design D3; `cargo nextest list -E` of each filter selects the same tests as before
- [ ] 2.5 Rewrite the justfile's `voice_smoke_binaries`, `e2e_smoke_binaries`, `smoke_parts_tests`, `e2e_seams_tests`, `_smoke-part-filter`, their self-tests and the `stage_inputs` lines naming `app/tests` files per design D3; `just _test-lock-phases`'s smoke-part self-test passes
- [ ] 2.6 Rewrite `app/tests/size_baseline.toml` and `.config/analyzers/{clippy,metrics}.toml` with `just size-baseline` and `just analyzers-baseline`; compare old and new entries by value after the path mapping (design D7) and confirm nothing grew or was added but the moved directory line
- [ ] 2.7 `just test` green with the same test count; commit

## 3. The `it` binaries of command-spotting and e2e-test

- [ ] 3.1 Red: extend the guard test to every member with a `tests/` directory; run it and see it fail naming the command-spotting and e2e-test files
- [ ] 3.2 Green: move `command-spotting/tests/{engine_it,placed_commands_it}.rs`, `server_it/`, `stall_it/` into `command-spotting/tests/it/` with `common`, `scripted_hearing` and `stream_clock` as modules of its `main.rs`; move `e2e-test/tests/{confirmation_time,hang_probe}.rs` into `e2e-test/tests/it/`; the guard passes
- [ ] 3.3 Rewrite the command-spotting filters of `.config/nextest.toml` (engine and placed-commands groups, priorities) and the justfile's `placed_commands_tests` per design D3; `cargo nextest list -E` of each selects the same tests
- [ ] 3.4 Rewrite the baselines for the moved paths as in 2.6; `just test` green with the same test count; commit

## 4. Debug info of the test profile

- [ ] 4.1 Measure design D5's `debug = "line-tables-only"` for the workspace crates (D6's build after a touch); keep it when faster, record the numbers in design D5
- [ ] 4.2 Measure `debug = false` for the dependencies the same way; keep it when faster, record the numbers
- [ ] 4.3 `just test` passes with the profile settings kept; commit

## 5. Docs and the acceptance measurement

- [ ] 5.1 Update `AGENTS.md` (§ Gherkin table, § Test layout, § Fail fast and § The user's voice binary names) and `docs/project-map.md` to the `tests/it/<adapter>.rs` layout
- [ ] 5.2 Measure the build after the last move as in 1.1 and the test count; record them in proposal.md § Why and confirm the acceptance bar (CPU sum at most a third, wall clock not longer, 1517 run and 1 skipped)
- [ ] 5.3 `just test` and `just check` pass on the final layout; commit

## 6. Version

- [ ] 6.1 `version` in `app/Cargo.toml` stays 0.37.1 (no release); manual checks: none
