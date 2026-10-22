## Why

`just test` runs after every edit in every change session, and its build, not its tests, takes most of its time. Measured on `main` on 2026-10-22:

- Incremental test build after touching one source file in `app/`: 3 min 31 s wall clock, 1 min 10 s CPU.
- Running all 1517 tests afterwards: 33 s.
- Stage journal for 2026-10-22: 152 `just test` runs, median 89 s, 90th percentile 171 s, longest 346 s.

Two causes:

1. Every file in `app/tests/*.rs` (about 38) is its own integration test target, so a one-file edit in `app` relinks about 40 separate executables, each with full debug info.
2. The build runs under `taskpolicy -c utility`, so when parallel sessions load the machine (load average about 100 on 10 cores during the measurement, mostly Miri runs from another session) it waits for CPU: 211 s wall against 70 s CPU.

The user wants the build sped up by tens of times: "Нахрена нам 40 бинарников?"

## What Changes

- Integration tests of each workspace member build into one test binary (`tests/it/main.rs` with a module per former file) instead of one binary per file.
- The test/dev profile carries less debug info (for example `debug = "line-tables-only"` for workspace crates and `debug = 0` for dependencies, `split-debuginfo = "unpacked"` on macOS), so linking is faster.
- The `utility` QoS clamp on the test build is measured with and without load; it is kept only if it does not slow the build itself.
- Everything that names test binaries keeps working: nextest test groups, priorities and `binary(...)` filters in `.config/nextest.toml`, the justfile's filters and `stage_inputs`, the `test@<member>` part keys, `rerun-failed`, the architecture, size budget and unsafe impl tests.

Current stage: PoC. Target stage: PoC.
Scope: the test targets layout of `app`, `command-spotting`, `copy-pool` and `e2e-test` (except the E2E Gherkin harness, which may stay its own binary), Cargo profiles, nextest config and the justfile recipes that name test binaries.
Acceptance bar: the incremental `just test` build after a one-file edit in `app/` is at least 10 times faster than the 2026-10-22 baseline, measured before and after on the same machine with no other load and recorded in the stage journal; the test count is the same before and after (1517 run, 1 skipped) and every test passes.
Out of scope: the test contents themselves, the E2E and model stages, the machine lock, release build profiles.

## Capabilities

### Modified Capabilities

- `test-infrastructure`: test targets build into one binary per member, with a lighter debug profile, so `just test` after an edit is dominated by running tests, not linking them.

## Impact

- `app/tests/`, `command-spotting/tests/`, other members' `tests/` directories: moved under one `tests/it/` per member.
- `Cargo.toml` (workspace profiles), member `Cargo.toml` `[[test]]` entries.
- `.config/nextest.toml`: `binary(...)` filters rewritten to the new binary plus `test(...)` module paths.
- `justfile`: `_test`, rerun filters, smoke/E2E selections that name test binaries, `stage_inputs`.
- `app/tests/size_baseline.toml`, `.config/analyzers/*.toml` entries that name moved files.

## Open points

- Whether one binary for all of `app` or a few grouped binaries (fast unit-ish, macOS UI, model-loading) is faster overall; proposed: one, split only if measurement shows a reason.
- Tests that need process isolation (ONNX Runtime aborting on several threads per process, the shared pasteboard): nextest still runs each test in its own process, so one binary should keep that; confirm.
- Exact debug level: proposed `line-tables-only` for workspace crates so panics keep file and line, `0` for dependencies.
- Whether to drop the `utility` QoS clamp; proposed: decide by measurement under load.
- Whether `sccache`, a faster linker or `cargo` build cache tweaks belong here to reach 10x; proposed: try them only if the two main parts fall short.

## Release

SemVer: no release — a change to the test build only, no behavior change of the app.
Target version: 0.37.0.
