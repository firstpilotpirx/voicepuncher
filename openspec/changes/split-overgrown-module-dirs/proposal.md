## Why

The size budgets (a module directory at most 2 000 code lines, a file at most 300) have exceptions: `app/tests/size_baseline.toml` lists the units over budget on `main`, and a listed unit may only shrink. Every change that adds a line to a listed unit fails `just static` and must first find lines to cut elsewhere. In `widen-recording-pill` (2026-10-24) three of five `just test` runs failed on these budgets (a file at 309 of 300 lines, the overlay directory +2 lines, the steps directory +1 line), cost about 3 minutes and pushed a one-step addition into an odd nested file (`visual_feedback/overlay/width.rs`). The user wants the exceptions gone: split the code so it fits the budgets, instead of living with a list.

Today's list holds 8 directories and 52 files:

- directories: `app/tests/steps` 7 067, `app/tests/it` 5 604, `app/src/core/application` 3 352, `app/src/adapters/secondary/visualfeedback/macos` 2 757, `e2e-test/tests/steps` 2 335, `app/src/core/domain` 2 274, `app/src/adapters/primary/settings/macos` 2 085, `command-spotting/tests/it` 2 047;
- 52 files over 300 code lines (`[file_lines]`).

## What Changes

- Split every listed directory into subdirectories (by capability or concern) so each holds at most 2 000 code lines in its own `.rs` files.
- Split every listed file into files of at most 300 code lines.
- Remove every `[directory_lines]` and `[file_lines]` entry from `app/tests/size_baseline.toml`; add none. Move `visual_feedback/overlay/width.rs` back beside the other overlay steps.
- Update the paths in `.config/stages.toml` (`stage_inputs`, `recognition_paths`, `chunking_paths`) and in `app/tests/it/main.rs`-style module lists for every moved file.

Current stage: PoC. Target stage: PoC.
Scope: file moves, module declarations and splitting files along existing seams; no behavior change.
Acceptance bar: `just test` and the landing queue's `full-check` pass with no `[directory_lines]` or `[file_lines]` entry left in `app/tests/size_baseline.toml`.
Out of scope: the function, nesting, parameter and struct entries (change `clear-size-baseline-functions`), the clippy and metrics ratchets (change `clear-analyzer-ratchets`).

## Capabilities

### Modified Capabilities

- None: a refactor without behavior change.

## Impact

- `app/tests/steps/**`, `app/tests/it/**`, `app/src/core/application/**`, `app/src/core/domain/**`, `app/src/adapters/secondary/visualfeedback/macos/**`, `app/src/adapters/primary/settings/macos/**`, `e2e-test/tests/steps/**`, `command-spotting/tests/it/**`, the 52 listed files
- `app/tests/size_baseline.toml`, `.config/stages.toml`, `app/tests/it/architecture.rs` (moved module paths)

## Open points

- Split lines: proposed by capability for test folders (voice control, speech recognition, visual feedback, services, rest) and by concern for `core/application` and `core/domain`; the design picks them per directory.
- The change is large (60 units); proposed: one task group per directory, each its own commit, so a red group is easy to find.

## Release

SemVer: no release — refactor without behavior change.
Target version: 0.38.5.
