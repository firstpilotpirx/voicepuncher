## Status: REJECTED (2026-10-22)

The user rejected this change before any of it merged: mutation testing, coverage floors and dynamic analyzers are not wanted in this project. The baseline run (cargo-mutants over app/src/core, 1 018 mutants, 715 rebuild-and-test loops in 3.5 h) was killed, the branch chore/measure-test-strength was abandoned, and the change was removed from the backlog (#291). Do not revive it, do not file a similar measurement-only change, and never run `cargo-mutants` here. The user's words: «полная хрень», «мне такая таска не нужна», «прекрати столько запускать тестов».

## Why

The project has 1 408 tests and a test-to-code ratio of ≈ 1.2 : 1, but nothing measures whether they would catch a regression (audit of 2026-10-22): ceremony tests exist (`unmask-gated-tests`), many mock assertions check a call happened but not its argument, and no coverage or mutation score is recorded. The kit's `tech-matrix-rust.md` names coverage (`cargo-tarpaulin`, which does not run on Apple Silicon) and dynamic analyzers (miri, sanitizers) that the project never ran.

## What Changes

- Coverage: `cargo-llvm-cov` (works on macOS arm64) runs in `full-check` over `just test` (no models), writes the report into the stage journal and `~/.cache/voicepuncher/`, and a per-crate line and region coverage floor (branch coverage needs nightly) is ratcheted like the accuracy baseline.
- Mutation testing: `cargo-mutants` over `app/src/core` and `copy-pool` (no services) with a caught-mutant floor per crate; the survivors it lists are the next tests to write. In `full-check` it runs `copy-pool` whole (151 mutants, 2 min) and in `core` only the mutants of the lines the branch changed (measured 2026-10-22: every `core` mutant rebuilds the `app` crate, ≈ 20–50 s each, so the 1 018 mutants take hours); the whole `core` runs on demand (`just strength-baseline`, hours in the background) to set the floor, and `just mutants <path>` runs one path.
- Dynamic analysis where it applies: `miri` over the unsafe-free `core` unit tests, and an address sanitizer run of `main_queue.rs`, `focus/macos`, `textpasting/macos` integration tests under a `just sanitize` recipe (nightly toolchain pinned for that recipe only).
- `AGENTS.md` § How to verify a change names the floors and the recipes.

Current stage: PoC. Target stage: PoC.
Scope: the recipes, the floors, the first batch of tests written against the survivors `cargo-mutants` reports in `core/domain` and `core/application`.
Acceptance bar: `just full-check` reports coverage and the mutation score of the branch's mutants and fails under the floors; the floors are in a baseline file with the commit they came from; the first survivors in `core/domain` are killed.
Out of scope: coverage of AppKit adapters (manual run); changing the tests' structure.

## Capabilities

### Modified Capabilities

- `change-workflow`: coverage and mutation floors as stages.

## Impact

- `justfile` (the pinned nightly for `just sanitize` only), `.config/nextest.toml`, `.config/analyzers.toml`, `analyzers/`, `AGENTS.md`, a baseline file, new unit tests in `app/src/core/**`.

## Open points

- Mutation scope: `core` only (fast) or every crate (`command-spotting` has ONNX tests, slow); decided (scope call of 2026-10-22): `copy-pool` whole and the branch's changed lines of `core` in `full-check`, the whole `core` on demand, the rest on demand.

## Release

SemVer: no release.
Target version: unchanged.
