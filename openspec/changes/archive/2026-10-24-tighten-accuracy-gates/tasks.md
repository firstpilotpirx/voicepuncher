## 1. The gate

- [x] 1.1 Red: unit tests in `e2e-test/src/accuracy_gate.rs` — worse by more than the margin fails naming rate, baseline, margin and commit; within the margin passes; clearly better passes with the `just accuracy-baseline` hint; no baseline for the sample size fails; margin 0.01 below 10 rates; margin `max(0.01, 1.5σ)` of the last 10
- [x] 1.2 Green: `baseline`, `history`, `margin`, `judge` and their messages
- [x] 1.3 The step `the mean word error rate is no worse than the accepted baseline` reads `e2e-test/accuracy_baseline.json` and the journal, calls the gate and appends `wer <sample> <mean>` to `VOICEPUNCHER_STAGES_REPORT`; `MAX_MEAN_WER` removed; `file_transcription.feature` uses the new step
- [x] 1.4 `just test`; commit

## 2. The journal, the baseline recipe and the trend

- [x] 2.1 Red: `journal/tests.rs` — a report with `wer 100 0.047` gives the journal line `"wer":{"sample":100,"mean":0.047}`, no such line gives `"wer":null`
- [x] 2.2 Green: `Report` keeps the last `wer` line; the journal line's 15th key
- [x] 2.3 Red: `accuracy_baseline.rs` — the last rate of this worktree per sample size replaces its entry, another worktree's lines are ignored, an unmeasured size keeps its entry
- [x] 2.4 Green: `dev-tools accuracy-baseline` and the `justfile` recipe `accuracy-baseline`
- [x] 2.5 Red: the `stage-times` summary lists the rates per sample size with commits, oldest first
- [x] 2.6 Green: the `accuracy` section of the summary
- [x] 2.7 `just test`; commit

## 3. Baseline and documents

- [x] 3.1 `e2e-test/accuracy_baseline.json` with the rates measured on `main` (100: 0.047; 10: from `just e2e-quick`) and commit 5ee5fbd
- [x] 3.2 `AGENTS.md` § How to verify a change, `README.md` and the `justfile` comments: the ratchet, the margin, `just accuracy-baseline`, the trend; no "30 %"
- [x] 3.3 `just test`; commit
