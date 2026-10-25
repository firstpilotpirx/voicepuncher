## Why

The size budgets and the strict analyzers keep lists of exceptions that may only shrink, so every change touching a listed unit fails `just static` until it cuts lines elsewhere. Frozen and `@wip` tests leave behavior unchecked. The user wants every exception removed by real refactoring and every disabled test fixed or deleted, in one change; it absorbs `clear-size-baseline-functions`, `clear-analyzer-ratchets` and `fix-or-drop-disabled-tests`.

## What Changes

- Split the 8 directories and 52 files of `app/tests/size_baseline.toml` to fit 2 000 and 300 code lines.
- Refactor its 78 function, nesting, parameter and struct entries; no `reason = "size baseline"` left; the file stays, empty.
- Fix the 571 findings of `.config/analyzers/clippy.toml` (`cargo clippy --fix` first); turn off `cast_precision_loss` and `suboptimal_flops` in `.config/analyzers.toml` with their reasons.
- Bring the 97 functions under cyclomatic 10, break the 3 import cycles, remove the 1 235 duplicated lines; keep only the `unsafe` count of `app` (macOS calls).
- Fix the 2 shellcheck findings of `.config/analyzers/recipes.toml`.
- For each of the 4 frozen tests and 4 frozen or unmet `@wip` scenarios: fix and enable it, or delete it and name the test that covers its requirement.
- Update the moved paths in `.config/stages.toml` and the module lists.

Current stage: PoC. Target stage: PoC.
Scope: refactors, lint configuration, the disabled tests and the code their fixes need.
Acceptance bar: `size_baseline.toml`, `clippy.toml`, `recipes.toml` and `metrics.toml` (but `unsafe app`) hold no entry; `just frozen-tests` lists none; two consecutive `full-check` runs pass.
Out of scope: the manual `@wip` scenarios of `app_distribution.feature`, the `#[ignore]` measurement in `e2e-test/tests/it/confirmation_time.rs`.

## Capabilities

### Modified Capabilities

- None: the checks' mechanism stays; a test fix that changes a spec's behavior adds its delta spec.

## Impact

- code and tests of every workspace member, `.config/`, the size baseline, the E2E features

## Release

SemVer: PATCH when a test fix changes the app's code, else no release.
Target version: 0.38.6 (or no release).
