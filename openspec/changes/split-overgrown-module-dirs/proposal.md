## Why

Two module directories sit far over the 2 000-line directory budget and are listed in `app/tests/size_baseline.toml`: `app/tests/steps` (7 067 code lines) and `app/src/adapters/secondary/visualfeedback/macos` (2 757). A listed unit may only shrink, so every change that adds even one line there fails `just static` and must first find lines to cut. In `widen-recording-pill` (2026-10-24) three of five `just test` runs failed on these budgets (a file at 309 of 300 lines, the overlay directory +2 lines, the steps directory +1 line), cost about 3 minutes and pushed a one-step addition into an odd nested file (`visual_feedback/overlay/width.rs`).

## What Changes

- Split `app/tests/steps` into subdirectories per capability group so each directory stays under 2 000 code lines, and split `visualfeedback/macos` (pill content, waveform, glass, probe) the same way.
- Drop the two directory entries from `app/tests/size_baseline.toml`; move `width.rs` back next to the other overlay steps.

Current stage: PoC. Target stage: PoC.
Scope: file moves and module declarations only; no behavior change.
Acceptance bar: `just test` passes with both directory entries gone from the size baseline and no new entry added.
Out of scope: shrinking single files or functions listed in the baseline.

## Capabilities

### Modified Capabilities

- None: a refactor without behavior change.

## Impact

- `app/tests/steps/**`, `app/src/adapters/secondary/visualfeedback/macos/**`, `app/tests/size_baseline.toml`, `.config/stages.toml` (`stage_inputs` paths of the moved files)

## Open points

- The split lines for the steps directory: proposed by capability (voice control, speech recognition, visual feedback, services, rest).

## Release

SemVer: no release — refactor without behavior change.
Target version: 0.38.5.
