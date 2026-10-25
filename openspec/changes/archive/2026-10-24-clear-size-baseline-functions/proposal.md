## Why

Besides directories and files, `app/tests/size_baseline.toml` lists 78 smaller units over budget on `main`: 33 functions over 40 code lines, 26 over nesting 4, 12 over 5 parameters and 7 structs over 12 fields; each listed function carries `#[expect(clippy::<lint>, reason = "size baseline")]`. A listed unit may only shrink, so any change that touches one must cut it first. The user wants the exceptions gone: refactor the code so it fits the budgets.

## What Changes

- Refactor every listed function, nesting, parameter list and struct to fit its budget (extract functions, parameter objects, early returns, split structs by concept).
- Remove every remaining entry of `app/tests/size_baseline.toml` and every `#[expect(..., reason = "size baseline")]`; add none, so the file ends empty (or is deleted together with its reader's handling of a missing file, as the design decides).

Current stage: PoC. Target stage: PoC.
Scope: refactors without behavior change of the listed units.
Acceptance bar: `just test` and the landing queue's `full-check` pass with `app/tests/size_baseline.toml` empty and no `reason = "size baseline"` left in the code.
Out of scope: directories and files (change `split-overgrown-module-dirs`), the clippy and metrics ratchets (change `clear-analyzer-ratchets`).

## Capabilities

### Modified Capabilities

- None: a refactor without behavior change.

## Impact

- the 78 listed units across `app/`, `command-spotting/`, `e2e-test/`; `app/tests/size_baseline.toml`

## Open points

- Whether to keep `size_baseline.toml` as an empty file for future merges of old branches: proposed keep it empty.
- Grouping: proposed one task group per crate area (core, adapters, tests, command-spotting, e2e-test).

## Release

SemVer: no release — refactor without behavior change.
Target version: 0.38.5.

## Absorbed by

split-overgrown-module-dirs.

The user wants every exception and every disabled test cleared in one change run in one session, so this work moved into the umbrella change `split-overgrown-module-dirs`, which owns its scope and acceptance bar.
