## Why

The project has 1 408 tests and a test-to-code ratio of ≈ 1.2 : 1, but nothing measures whether they would catch a regression (audit of 2026-10-22): ceremony tests exist (`unmask-gated-tests`), many mock assertions check a call happened but not its argument, and no coverage or mutation score is recorded. The kit's `tech-matrix-rust.md` names coverage (`cargo-tarpaulin`, which does not run on Apple Silicon) and dynamic analyzers (miri, sanitizers) that the project never ran.

## What Changes

- Coverage: `cargo-llvm-cov` (works on macOS arm64) runs in `full-check` over `just test` (no models), writes the report into the stage journal and `~/.cache/voicepuncher/`, and a per-crate line and branch coverage floor is ratcheted like the accuracy baseline.
- Mutation testing: `cargo-mutants` over `app/src/core` and `copy-pool` (fast, no services) with a caught-mutant floor per crate; the survivors it lists are the next tests to write; it runs in `full-check` (minutes) and on demand (`just mutants <path>`).
- Dynamic analysis where it applies: `miri` over the unsafe-free `core` unit tests, and an address sanitizer run of `main_queue.rs`, `focus/macos`, `textpasting/macos` integration tests under a `just sanitize` recipe (nightly toolchain pinned for that recipe only).
- `AGENTS.md` § How to verify a change names the floors and the recipes.

Current stage: PoC. Target stage: PoC.
Scope: the recipes, the floors, the first batch of tests written against the survivors `cargo-mutants` reports in `core/domain` and `core/application`.
Acceptance bar: `just full-check` reports coverage and the mutation score and fails under the floors; the floors are in a baseline file with the commit they came from; the first survivors in `core/domain` are killed.
Out of scope: coverage of AppKit adapters (manual run); changing the tests' structure.

## Capabilities

### Modified Capabilities

- `change-workflow`: coverage and mutation floors as stages.

## Impact

- `justfile`, `.config/nextest.toml`, `rust-toolchain.toml` (a nightly pin for sanitizers only), `AGENTS.md`, a baseline file, new unit tests in `app/src/core/**`.

## Open points

- Mutation scope: `core` only (fast) or every crate (`command-spotting` has ONNX tests, slow); proposed: `core` and `copy-pool` in `full-check`, the rest on demand.

## Release

SemVer: no release.
Target version: unchanged.
