## Why

The accuracy gate does not gate a regression (audit of 2026-10-22): the WER threshold is 0.30 (`e2e-test/tests/steps/file_transcription.rs`, `MAX_MEAN_WER`), far above what large-v3-turbo reaches on the corpus (about 0.05 on the fixed 100 recordings), so a model, prompt or chunking regression up to that level passes. The stage journal keeps every run's accuracy choice but not the WER it measured, so there is neither a baseline nor a trend. (The refinement score this change also meant to gate went with `remove-llm-refinement`.)

## What Changes

- The mean WER is ratcheted: a baseline file (`e2e-test/accuracy_baseline.json`) holds the accepted mean WER per sample size (10 and 100) with the commit it came from; an accuracy run fails when its mean WER is worse than the baseline by more than the margin, and passes with a hint to update the baseline when it is better by more than the margin.
- The margin is 0.01 WER until the stage journal holds 10 runs of that sample size; from then on it is 1.5 × the standard deviation of the last 10, never below 0.01.
- Every accuracy run records its sample size and mean WER in the stage journal (`wer`).
- `just accuracy-baseline` writes the baseline from this checkout's last journaled run of each sample size, with its commit; a baseline that gets worse is reviewed in the PR like code.
- `just stage-times` shows the trend of the mean WER per sample size.
- `MAX_MEAN_WER` and the "below 30 %" scenario go; `AGENTS.md` § How to verify a change states the ratchet and how to accept a baseline move.

Current stage: PoC. Target stage: PoC.
Scope: `e2e-test/src/accuracy_gate.rs` (new), `e2e-test/tests/steps/file_transcription.rs`, `e2e-test/tests/features/file_transcription.feature`, `e2e-test/accuracy_baseline.json`, `dev-tools` (journal `wer`, `accuracy-baseline`, the trend in `stage-times`), `justfile`, `AGENTS.md`, `README.md`.
Acceptance bar: a unit test of the gate shows a mean WER 0.02 above the baseline fails and one within the margin passes; the accuracy stages pass on this branch against the committed baseline; the baseline is in the repository with the commit it came from.
Out of scope: the models and prompts themselves; the corpus; the voice control accuracy scenario (20 recordings of ordinary speech, no WER).

## Capabilities

### Modified Capabilities

- `file-transcription`: the accuracy bar is the accepted baseline, not a fixed 30 %.
- `change-workflow`: the stage journal records the mean WER, `just accuracy-baseline` accepts a new baseline, `just stage-times` shows the trend.

## Impact

- `e2e-test/**`, `dev-tools/**`, `justfile`, `AGENTS.md`, `README.md`, `openspec/specs/{file-transcription,change-workflow}`.

## Open points

- Baseline format: JSON instead of the TOML first proposed, since both crates already read JSON and neither reads TOML (no new dependency).

## Release

SemVer: no release — test gates only.
Target version: unchanged.
