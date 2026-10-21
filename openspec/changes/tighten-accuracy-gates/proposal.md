## Why

The accuracy gates do not gate a regression (audit of 2026-10-22): the WER threshold is 0.30 (`e2e-test/tests/steps/file_transcription.rs:17`, `MAX_MEAN_WER`), far above what large-v3-turbo reaches on the corpus, so a model, prompt or chunking regression up to that level passes; the refinement evaluation decides by verdicts (`e2e-test/src/refinement_eval.rs:139–165`, `evaluation.rs:107–119`) and prints a score that gates only "above raw" for two categories (`evaluation.rs:123–150`), so a worse prompt or model passes while every verdict holds. The stage journal keeps every run's accuracy sample but no baseline.

## What Changes

- WER and the refinement score are ratcheted: a baseline file holds the last accepted mean WER per sample size and the refinement score per category; a run fails when it is worse than the baseline by more than a stated margin (noise measured over the journal), and a change that improves them updates the baseline in the same PR.
- The thresholds are derived from the baseline, not constants; `MAX_MEAN_WER` goes.
- `just stage-times` (or `just accuracy-trend`) prints the trend of both from the journal, so a slow drift is visible.
- `AGENTS.md` § How to verify a change states the ratchet and how to accept a baseline move.

Current stage: PoC. Target stage: PoC.
Scope: `e2e-test/src/{refinement_eval,evaluation,wer}.rs`, `e2e-test/tests/steps/{file_transcription,text_refinement}.rs`, the baseline file, the `justfile` recipes that read it, `AGENTS.md`.
Acceptance bar: a run with an injected 2-point WER loss (a worse prompt in a test profile) fails; a run on `main` passes; the baseline is in the repository with the commit it came from.
Out of scope: the models and prompts themselves; the corpus.

## Capabilities

### Modified Capabilities

- `speech-recognition`: accuracy is gated against a baseline.
- `text-refinement`: the refinement score is gated against a baseline.

## Impact

- `e2e-test/**`, `justfile`, a baseline file (`e2e-test/accuracy_baseline.toml`), `AGENTS.md`, `openspec/specs/{speech-recognition,text-refinement}`.

## Open points

- Margin: proposed 1.5 × the standard deviation of the last 10 passing runs in the journal, floor 0.01 WER.

## Release

SemVer: no release — test gates only.
Target version: unchanged.
