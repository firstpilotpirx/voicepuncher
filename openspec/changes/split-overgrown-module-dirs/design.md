# Design

## Context

The size analyzer counts a directory as the code lines of the `.rs` files directly in it; ports must stay flat in `app/src/core/application/` (architecture test R8). Every listed unit and finding is keyed by its path, so a moved file changes its keys.

## Decisions

1. **Directories shrink by moving a module's root file into its folder.** Where `x.rs` sits beside `x/`, it becomes `x/mod.rs`; flat files of one concern move into a subfolder named after it (`app/tests/it/`: `commandspotting/`, `transcription/`, `localservices/`, `macos/` and so on, each listed in `main.rs`). In `core/application` only the non-port modules move (`chunking/`, `model_download/`, `text_insertion/`); every port trait stays flat. Rejected: a higher directory budget.
2. **Files split along existing seams**: tests beside their module (`tests.rs`, `tests/<topic>.rs`), cases tables into their own file, macOS view code per view. No file is split by line count alone.
3. **Baselines are rewritten after every group** (`just size-baseline`, `just analyzers-baseline`): a moved unit may appear under its new key in between; the final commit holds none. The files stay, empty, for merges of older branches.
4. **`cargo clippy --fix` first**, then hand fixes. `cast_precision_loss` (sample and frame counts into `f32`/`f64`, never near 2^24 in practice) and `suboptimal_flops` (`mul_add` rewrites read worse in audio math and change no result that matters) are turned off in `.config/analyzers.toml` with these reasons. Every other cast goes through `try_from` or a named conversion helper per crate that states its range; `expect`/`panic` outside tests return errors.
5. **Cyclomatic complexity** falls by extracting per-case functions and lookup tables; **import cycles** break by moving the shared type down into the module both sides use (`core/application` ↔ `pipeline`, `command-spotting` `confirm`/`engine`/`worker`, `dev-tools` `keys`).
6. **Duplicated lines** go into shared helpers: test steps into `test-steps` or the harness's `shared/`, integration tests into a `common` module per binary.
7. **`unsafe app` stays** listed: its sites are Objective-C and CoreAudio calls that have no safe wrapper; each already carries `SAFETY`. It is the only entry left.
8. **Disabled tests**: each gets a root cause before any edit. A fix in the code under test is test-first: the unfrozen test is the Red. A timing bound is kept and the slow path fixed; only when the cause is the machine's load and no code change makes it reliable is the test deleted, with the covering test named here. A test green without a fix stays only after two consecutive `full-check` runs pass (`kit verify --series 2`).

## Test levels

| What | Level |
|---|---|
| refactors (decisions 1–7) | the existing unit, integration, architecture and slice/component tests, unchanged in meaning |
| chunk request hang | slice/component `.feature` |
| recorded restart restart, chain ending in start, cue of restart restart, regression examples | E2E `.feature` (`voice_control`) |
| Russian speech repeated by recognition | E2E `.feature` (`local_services`, `@resilience`) |
| pinned engines are staged | E2E `.feature` (`app_distribution`, bundle stage) |

## Risks / Trade-offs

- One very large branch: a red group is found by its commit; groups run in an order where moves come before refactors, so a refactor never chases a moved path.
- Moved test files change stage keys: most stages run again once.
- A flaky E2E scenario may still flake on the two full-check runs; then it is fixed again or deleted, never frozen.
