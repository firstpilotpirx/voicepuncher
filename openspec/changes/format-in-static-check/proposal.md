## Why

The first change after `speed-up-test-loop` (`widen-recording-pill`, 2026-10-24) ran `just test` five times; four runs failed in the `static` stage. The first of them failed only on formatting: a new step written by hand was not `rustfmt`-clean, and `static` printed the diff and stopped. Formatting is the one finding the tool can fix by itself, yet it costs a full round trip (read the log, run `cargo fmt`, run again).

## What Changes

- `just static` (and so the first stage of `test`, `check` and `full-check` run locally) formats the workspace in place before its other checks, and prints the files it reformatted, instead of failing on a formatting diff.
- Hosted CI and the landing queue keep failing on a formatting diff (`cargo fmt --check`), so unformatted code still never lands.

Current stage: PoC. Target stage: PoC.
Scope: the formatting step of the `static` stage in a local run.
Acceptance bar: a test proves a local `just static` on a tree with an unformatted file reformats it, names it and goes on to clippy; a CI run (`CI` set) still fails on the same tree.
Out of scope: clippy, the size budgets, the metrics.

## Capabilities

### Modified Capabilities

- `change-workflow`: the static check formats the code itself in a local run.

## Impact

- `justfile` (`_static`), `dev-tools/` (static stage), `dev-tools/tests/features/change_workflow.feature`

## Open points

- A stage key is computed before the stage runs: reformatting changes the inputs during the run, so that `static` pass is not recorded. Proposed: accept it (the next run passes and records).

## Release

SemVer: no release — tooling only.
Target version: 0.38.5.
