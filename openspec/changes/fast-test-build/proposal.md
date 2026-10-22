## Why

`just test` runs after every edit in every change session, and its build, not its tests, takes most of its time. Measured on `main` on 2026-10-22:

- Incremental test build after touching one source file in `app/`, under a load average of about 100 (the other sessions' builds and Miri runs): 3 min 31 s wall clock, 1 min 10 s CPU on the first measurement; 10 s wall clock and 70 s CPU on the explore measurement of the same day, with the `utility` QoS clamp (11 s) and without it (9 s).
- Running all 1517 tests afterwards: 33 s.
- Stage journal for 2026-10-22: 152 `just test` runs, median 89 s, 90th percentile 171 s, longest 346 s.

Where the 70 s of CPU go (`cargo build --timings`): about 45 s link the 41 test binaries of `app` (one per file in `app/tests/`, about 1 s each), 6 s compile and link the unit tests of `app`, 6 s the E2E Gherkin harness, 2.4 s the `e2e-test` library that every `app` test binary links (a dev-dependency cycle `app ↔ e2e-test`). The critical path on an idle machine is the `app` crate with its unit tests, about 8 s; the 41 links are CPU, disk and memory that parallel sessions and Miri runs fight over, which is where the 3.5 min came from.

The user wants the build sped up: "Нахрена нам 40 бинарников?"

## What Changes

- Integration tests of each workspace member build into one test binary (`tests/it/main.rs` with a module per former file) instead of one binary per file: `app` 37 files → one `it`, `command-spotting` 4 → one, `e2e-test` 2 helpers → one. The Gherkin harnesses (`cucumber_integration`, `cucumber_e2e`) stay their own binaries.
- The three macOS main-thread harnesses of `app` (overlay, menu bar, settings window; `harness = false`, three identical `main()` copies) build into one `harness = false` binary with one `main()`.
- A guard test proves every file under `tests/it/` is declared as a module of its `main.rs`, so a test file no module names cannot stay silently unbuilt.
- The test profile carries less debug info (`line-tables-only` for workspace crates, none for dependencies), each setting kept only when the measurement shows it shortens the build.
- The `utility` QoS clamp on the test build stays: the measurement under load showed no difference.
- Everything that names test binaries keeps working: nextest test groups, priorities and `binary(...)` filters in `.config/nextest.toml` (rewritten to the new binaries plus module paths), the justfile's smoke, seams and rerun selections and `stage_inputs`, the `test@<member>` part keys, `rerun-failed`, the architecture, size budget and unsafe impl tests; the baselines (`app/tests/size_baseline.toml`, `.config/analyzers/*.toml`) get the moved paths without a grown unit.

Current stage: PoC. Target stage: PoC.
Scope: the test targets layout of `app`, `command-spotting` and `e2e-test` (except the Gherkin harnesses), Cargo profiles, nextest config, the justfile recipes and docs that name test binaries.
Acceptance bar: measured before and after on the same machine, in the same worktree, with no other load, after touching one source file of `app/` (`cargo build --timings` of every test target): the CPU time summed over the build units is at most a third of the baseline, and the wall clock is not longer; recorded in the proposal; the test count is the same before and after (1517 run, 1 skipped) and every test passes.
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
Target version: 0.37.1.
