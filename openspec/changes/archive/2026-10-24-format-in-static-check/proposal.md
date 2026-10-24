## Why

The first change after `speed-up-test-loop` (`widen-recording-pill`, 2026-10-24) ran `just test` five times; four runs failed in the `static` stage. The first of them failed only on formatting: a new step written by hand was not `rustfmt`-clean, and `static` printed the diff and stopped, so the size-budget failure behind it showed only on the next run. A finding the tools can fix by themselves still costs a full round trip (read the log, run the fixer, run again). The user wants every finding the static check can fix mechanically fixed on the fly, formatting at least.

## What Changes

- A local `just static` (and so the first stage of `test`, `check` and `full-check` run locally) first applies every fix the tools can make mechanically, prints the files it changed, and only then runs its checks:
  - formatting: `cargo fmt --all` in place instead of `--check`;
  - clippy: the machine-applicable suggestions (`cargo clippy --fix`), then the usual `-D warnings` check for what is left;
  - any other mechanical fixer of the static check found while designing (for example the spelling fixes of `typos`, if it moves into `static`).
- Findings no tool can fix (size budgets, metrics, clippy findings without a machine-applicable fix) still fail the check as today.
- Hosted CI and the landing queue keep failing on anything unfixed (`cargo fmt --check`, clippy `-D warnings`), so unfixed code never lands; only a local run fixes.

Current stage: PoC. Target stage: PoC.
Scope: the mechanical fixes of the `static` stage in a local run.
Acceptance bar: tests prove that a local `just static` on a tree with an unformatted file and a machine-fixable clippy warning fixes both, names the files and goes on to the size check; a CI run (`CI` set) still fails on the same tree.
Out of scope: fixing size budgets or metrics; changing which lints run.

## Capabilities

### Modified Capabilities

- `change-workflow`: the static check fixes what the tools can fix mechanically in a local run.

## Impact

- `justfile` (`_static`), `dev-tools/` (static stage), `dev-tools/tests/features/change_workflow.feature`

## Open points

- A stage key is computed before the stage runs: fixes change the inputs during the run, so that `static` pass is not recorded. Proposed: accept it (the next run passes and records).
- `cargo clippy --fix` refuses a dirty tree without `--allow-dirty --allow-staged`, and a wrong machine-applicable fix is rare but possible. Proposed: allow it locally; the agent commits and reviews the diff like any other edit.

## Release

SemVer: no release — tooling only.
Target version: 0.38.5.
