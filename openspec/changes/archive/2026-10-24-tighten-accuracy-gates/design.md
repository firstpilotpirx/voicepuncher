## Context

The accuracy stages (`e2e-quick`: 10 recordings, `e2e-bench`: 100) run the `@accuracy` scenario of `file_transcription.feature`; its last step fails only above `MAX_MEAN_WER = 0.30`, while the model measures about 0.05. The stage runner (`dev-tools`) writes one journal line per run from the runner's report file (`VOICEPUNCHER_STAGES_REPORT`), which the stages append lines to; the journal has no WER.

## Goals / Non-Goals

**Goals:** a WER regression of 0.02 fails the accuracy stage; the margin follows the measured noise once there is enough of it; accepting a new baseline is one recipe; the trend is visible in `just stage-times`.

**Non-Goals:** changing the models, the prompt or the corpus; a WER gate for the voice control accuracy scenario.

## Decisions

1. **Baseline file `e2e-test/accuracy_baseline.json`**, `{"wer": {"10": {"mean": …, "commit": "…"}, "100": {…}}}`. JSON because `e2e-test` and `dev-tools` both read it with `serde_json` already; TOML would add a dependency to both. The file is under `e2e-test/`, so it feeds the accuracy stages' key: a new baseline reruns them.
2. **The gate lives in the library** (`e2e-test/src/accuracy_gate.rs`): `baseline(json, sample)`, `history(journal, sample)` (the last 10 rates of that sample size), `margin(history)` and `judge(mean, accepted, margin) → Within | Better | Worse`, plus the message of each. The step `the mean word error rate is no worse than the accepted baseline` only reads the files and calls it, so the acceptance bar ("baseline + 0.02 fails") is a unit test, not a model run.
3. **Margin:** 0.01 with fewer than 10 journaled rates of the sample size; else `max(0.01, 1.5 × σ)` of the last 10 (population standard deviation). The journal is `VOICEPUNCHER_STAGE_JOURNAL`, else `~/.cache/voicepuncher/stage-times.jsonl`, the same path `dev-tools` writes.
4. **Recording the rate:** the step appends `wer <sample> <mean>` to the report file when `VOICEPUNCHER_STAGES_REPORT` is set; `Report::parse` keeps the last such line, and the journal line gets a 15th key `wer`: `{"sample": n, "mean": x}` or `null`. A run without an accuracy stage writes `null`, so older lines and new ones read the same.
5. **`just accuracy-baseline`** → `dev-tools accuracy-baseline`: reads the journal, keeps the lines of this worktree (`worktree` equals the current directory) with a `wer`, takes the last per sample size, and rewrites the baseline file with those entries and their `commit`; a sample size this worktree never measured keeps its entry. It prints each entry it wrote, old → new.
6. **Trend:** `just stage-times` gets an `accuracy` section: per sample size, the last 10 rates of the period with their commits, oldest first.
7. **No baseline for a sample size fails** (a hint names `just accuracy-baseline`): a silent pass would let the gate disappear unnoticed.
8. **First baseline:** the rates measured on `main` at 5ee5fbd (no text refinement): 100 recordings 0.047 (the `full-check` of `remove-llm-refinement`), 10 recordings measured with `just e2e-quick` before the baseline was committed.

## Test mapping

| Scenario | Test |
|---|---|
| Benchmark corpus is transcribed within the accuracy bar (file-transcription) | E2E `file_transcription.feature` `@accuracy`, run by `e2e-quick` and `e2e-bench` |
| A run worse than the baseline fails (file-transcription) | unit `accuracy_gate.rs` `should_fail_a_run_worse_than_the_baseline_by_more_than_the_margin` |
| A run within the margin passes (file-transcription) | unit `accuracy_gate.rs` `should_pass_a_run_within_the_margin` |
| A clearly better run passes with a hint (file-transcription) | unit `accuracy_gate.rs` `should_pass_a_clearly_better_run_with_the_baseline_hint` |
| The margin follows the measured noise (file-transcription) | unit `accuracy_gate.rs` `should_take_the_margin_from_the_last_ten_rates` and `should_keep_the_floor_margin_below_ten_rates` |
| A sample size without a baseline fails (file-transcription) | unit `accuracy_gate.rs` `should_fail_a_sample_size_without_a_baseline` |
| An accuracy run is journaled with its rate (change-workflow) | unit `dev-tools` `journal/tests.rs` `should_journal_the_mean_wer_the_accuracy_stage_reported` |
| The baseline is accepted from the last run (change-workflow) | unit `dev-tools` `accuracy_baseline.rs` `should_accept_the_last_rate_of_this_worktree_and_keep_the_others` |
| The summary shows the accuracy trend (change-workflow) | unit `dev-tools` `journal/times` tests `should_show_the_accuracy_trend_oldest_first` |

## Risks / Trade-offs

- A margin of 0.01 on 10 recordings may be tighter than their noise: the 10-sample set is fixed and the decoding deterministic, so repeated runs on one commit measured the same rate; once 10 rates are journaled, σ takes over.
- The journal is per machine: on another machine the margin starts at the floor again; acceptable at PoC.
