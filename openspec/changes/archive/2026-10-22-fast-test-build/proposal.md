## Why

`just test` runs after every edit in every change session, and its build, not its tests, takes most of its time. Measured on `main` on 2026-10-22:

- Incremental test build after touching one source file in `app/`, under a load average of about 100 (the other sessions' builds and Miri runs): 3 min 31 s wall clock, 1 min 10 s CPU on the first measurement; 10 s wall clock and 70 s CPU on the explore measurement of the same day, with the `utility` QoS clamp (11 s) and without it (9 s).
- Running all 1517 tests afterwards: 33 s.
- Stage journal for 2026-10-22: 152 `just test` runs, median 89 s, 90th percentile 171 s, longest 346 s.

Where the 70 s of CPU go (`cargo build --timings`): about 45 s link the 41 test binaries of `app` (one per file in `app/tests/`, about 1 s each), 6 s compile and link the unit tests of `app`, 6 s the E2E Gherkin harness, 2.4 s the `e2e-test` library that every `app` test binary links (a dev-dependency cycle `app ↔ e2e-test`). The critical path on an idle machine is the `app` crate with its unit tests, about 8 s; the 41 links are CPU, disk and memory that parallel sessions and Miri runs fight over, which is where the 3.5 min came from.

The user wants the build sped up: "Нахрена нам 40 бинарников?"

Baseline of this change (task 1.1, branch `chore/fast-test-build` at the merge of `main` 140fda0, warm target directory, load average 27.5): after `touch app/src/core/domain/cleaned_text.rs`, `cargo build --workspace --tests --timings` ran 51 units in 11 s wall clock and 80.5 s of summed unit time; `just test` ran 1543 tests across 55 binaries, all passed, 1 skipped.

After the change (task 5.2, commit after the moves and the profile setting, same worktree, warm target directory, load average 10–13): the same touch built 11 units in 5 s wall clock and 20.8 s of summed unit time (the moves alone: 11 units, 11 s, 40.5 s; `line-tables-only` halves it again; `debug = false` for the dependencies changed nothing and was dropped); `just test` ran 1544 tests across 9 binaries, all passed, 1 skipped: the 1543 of the baseline plus the guard test. On the final commit (task 5.5, after the merge of `main` and the walk's unit test) `just test` ran 1558 tests, all passed, 1 skipped: 1543 + 13 that `main` brought + the 2 of this change. The acceptance bar holds: a quarter of the CPU time, half the wall clock.

## What Changes

- Integration tests of each workspace member build into one test binary (`tests/it/main.rs` with a module per former file) instead of one binary per file: `app` 37 files → one `it`, `command-spotting` 4 → one, `e2e-test` 2 helpers → one. The Gherkin harnesses (`cucumber_integration`, `cucumber_e2e`) stay their own binaries.
- The three macOS main-thread harnesses of `app` (overlay, menu bar, settings window; `harness = false`, three identical `main()` copies) build into one `harness = false` binary with one `main()`.
- A guard test proves every file under `tests/it/` is declared as a module of its `main.rs`, so a test file no module names cannot stay silently unbuilt.
- The test profile carries less debug info (`line-tables-only`), kept because the measurement shows it halves the build again; `debug = false` for the dependencies was measured and dropped (no difference).
- The `utility` QoS clamp on the test build stays: the measurement under load showed no difference.
- Everything that names test binaries keeps working: nextest test groups, priorities and `binary(...)` filters in `.config/nextest.toml` (rewritten to the new binaries plus module paths), the justfile's smoke, seams and rerun selections and `stage_inputs`, the `test@<member>` part keys, `rerun-failed`, the architecture, size budget and unsafe impl tests; the baselines (`app/tests/size_baseline.toml`, `.config/analyzers/*.toml`) get the moved paths without a grown unit.

Current stage: PoC. Target stage: PoC.
Scope: the test targets layout of `app`, `command-spotting` and `e2e-test` (except the Gherkin harnesses), Cargo profiles, nextest config, the justfile recipes and docs that name test binaries.
Acceptance bar: measured before and after on the same machine, in the same worktree, with no other load, after touching one source file of `app/` (`cargo build --timings` of every test target): the CPU time summed over the build units is at most a third of the baseline, and the wall clock is not longer; recorded in the proposal; the test count after is the count of the baseline above plus the two tests this change adds (the layout guard and the unit test of its walk) plus what `main` merged meanwhile (1543 + 2 + 13 = 1558 run, 1 skipped) and every test passes; when the machine is not idle (the orchestrator's other sessions), the load average is recorded and the CPU sum is the primary figure.
Out of scope: the test contents themselves, the E2E and model stages, the machine lock, release build profiles, the dev-dependency cycle `app ↔ e2e-test` (about 2.5 s on the critical path, a change of its own), `sccache` and other linkers.

## Capabilities

### Modified Capabilities

- `change-workflow`: the integration tests of a workspace member build into one test binary besides the Gherkin harnesses, and the checks fail on a test file that binary leaves out, so `just test` after an edit is dominated by compiling the edited crate, not linking test binaries.

## Impact

- `app/tests/*.rs` and `app/tests/*/main.rs` (except the harness roots), `command-spotting/tests/`, `e2e-test/tests/{confirmation_time,hang_probe}.rs`: moved under one `tests/it/` per member.
- `Cargo.toml` (workspace profiles), `app/Cargo.toml` `[[test]]` entries.
- `.config/nextest.toml`: `binary(...)` filters rewritten to the new binaries plus `test(...)` module paths.
- `justfile`: smoke, seams and placed-commands selections, `_smoke-part-filter` and its self-tests, `stage_inputs`.
- `app/tests/architecture.rs` (R4, R9: the expected test file of an adapter), `app/tests/size_baseline.toml`, `.config/analyzers/{clippy,metrics}.toml` entries that name moved files.
- `AGENTS.md` § Gherkin, § Test layout; `docs/project-map.md`.

## Open points

Settled in explore on 2026-10-22:

- One binary per member, not grouped binaries: the links are the cost, the critical path is the crate itself.
- Tests that need process isolation (ONNX Runtime, the shared pasteboard) keep it: nextest runs each test in a process of its own whatever the binary.
- Debug level: `line-tables-only` for workspace crates so panics keep file and line, `0` for dependencies; each kept only when measured faster.
- The `utility` QoS clamp stays (no measured difference).
- No `sccache`, linker or cache tweaks.

## Release

SemVer: no release — a change to the test build only, no behavior change of the app.
Target version: unchanged (0.37.2 on `main` after the merge; no release).
