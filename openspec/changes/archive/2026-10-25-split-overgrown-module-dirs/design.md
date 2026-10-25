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
8. **Disabled tests**: each gets a root cause before any edit. A fix in the code under test is test-first: the unfrozen test is the Red. A timing bound is kept and the slow path fixed; only when the cause is the machine's load and no code change makes it reliable, or the user declined the fix, is the test deleted, with the covering test named here. A test green without a fix stays only after two consecutive `full-check` runs pass (`kit verify --series 2`).

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

## Disabled tests: causes and outcomes

- **The first request of a chunk hangs** (slice/component): a test bug. The step counted the lines holding "did not answer in time" and the lines holding " sent no answer in " and added the counts. An attempt the copy pool gives up on logs one line holding both phrases, an attempt `ureq` times out logs one holding only the first. Both end at the same 2 s, so under load the pool's give-up sometimes came first and the one hung attempt counted twice. The step now counts each line once. No app code changes.
- **A chain ending in start gives its cue at once** (E2E, `@wip`): deleted. The synthesized voice says "Enter" so that it sounds like "and a", so the early checks can't hear it in time. The archived change `hear-enter-before-start` explored the fixes, and the user declined them on 2026-10-18. No spec scenario asks for this cue. Covered by "Start followed by the dictation at once gives its cue at once", which times the cue of a start followed by speech, and by the chain example "That is all, stop, enter, start", which proves the chain is carried out.
- **The user's recorded restart restart followed by the new dictation at once**, the chain example **"Okay, enter, clear"** and the cue scenario **Restart restart followed by the new dictation at once gives its cue at once** (E2E): enabled again unchanged. Each was frozen after failing next to another session's full-check, before the model stages had the machine to themselves (`run-e2e-alone-on-the-machine`). Each run alone in its stage passed (the cue within its 0.7 s bound), and the two full-checks of the acceptance bar run them again. No app code changes.
- **Russian speech that makes speech recognition repeat itself** (E2E, e2e-resilience) and **The pinned engines are staged** (bundle stage): enabled again unchanged. Each was frozen after failing next to another session's full-check: the first went over its 2 s confirmation bound while another run used the GPU, the second went over the bundle stage limit while another run was building. Today the model stages run alone on the machine. Each passed alone in its stage. `just engines` on its own takes about 14 s, because the release build is already done. The two full-checks of the acceptance bar run both again. No app code changes.
