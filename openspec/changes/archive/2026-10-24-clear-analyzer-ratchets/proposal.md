## Why

The static analyzers also keep exceptions: `.config/analyzers/clippy.toml` lists 286 findings of the strict clippy groups (`pedantic`, `nursery`, `unwrap_used`, `expect_used`, `panic`, …) left on `main`, and `.config/analyzers/metrics.toml` about 100 ratcheted metrics (functions over cyclomatic 10, import cycles, `unsafe` per crate, duplicated lines). Each may only shrink, so a change that touches such code trips over findings it did not write. The user wants the exceptions gone.

## What Changes

- Fix every finding listed in `.config/analyzers/clippy.toml` (casts, clones, `unwrap`/`expect` outside tests, …); where a lint does not fit the project, turn it off in `.config/analyzers.toml` with a one-line reason instead of keeping a list.
- Bring every function under cyclomatic 10, break every import cycle, remove the duplicated lines `jscpd` finds; keep the `unsafe` counts only where the code must be unsafe (each block already documented).
- Leave both baseline files empty; `just analyzers-baseline` keeps working for later merges.

Current stage: PoC. Target stage: PoC.
Scope: code fixes for the listed findings and lint configuration; no behavior change.
Acceptance bar: `just lint`, `just test` and the landing queue's `full-check` pass with `.config/analyzers/clippy.toml` and `.config/analyzers/metrics.toml` holding no finding (the `unsafe` counts excepted, each named in the design).
Out of scope: the size budgets (changes `split-overgrown-module-dirs`, `clear-size-baseline-functions`).

## Capabilities

### Modified Capabilities

- None: code quality only, no behavior change.

## Impact

- code across `app/`, `command-spotting/`, `e2e-test/`, `dev-tools/`; `.config/analyzers/*.toml`, `.config/analyzers.toml`

## Open points

- Many clippy findings can be fixed by `cargo clippy --fix` once change `format-in-static-check` lands; proposed: run it first, then fix the rest by hand.
- Which strict lints to turn off rather than fix: proposed decided in design, each with its count and reason.

## Release

SemVer: no release — no behavior change.
Target version: 0.38.5.

## Absorbed by

split-overgrown-module-dirs.

The user wants every exception and every disabled test cleared in one change run in one session, so this work moved into the umbrella change `split-overgrown-module-dirs`, which owns its scope and acceptance bar.
