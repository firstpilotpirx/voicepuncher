# Proposal

## Why

The QA gates take longer than they need to, and every change pays for them: `just full-check` runs before every merge, `just check` after every task group. Timing on an M1 Max with large-v3-turbo, from a clean build, measured each recipe on its own: `full-check` takes ≈ 17 min and `check` ≈ 5.5 min. Three spots account for most of that time:

| Step | Time | Share | Cause |
|---|---|---|---|
| `e2e-bench` (in `full-check`) | 549 s | 55 % of `full-check` | 300 corpus recordings transcribed one after another; speech recognition (~0.75 s) and refinement (~0.9 s) never overlap |
| `chunked_transcription_it` (in `e2e-smoke`, so in `check` and `full-check`) | 134 s | 40 % of `check` | the 30-case seam sweep against the real service runs on every `check`, although `e2e-seams` already covers chunk seams before a merge |
| corpus size | — | — | mean WER is 6.5 % against a 30 % bar; 300 recordings measure it far more precisely than the bar needs |

## What Changes

- **Accuracy benchmark in parallel:** the accuracy scenario transcribes two recordings at a time, so speech recognition of one overlaps refinement of the other. Expected: `e2e-bench` ≈ 9 → ≈ 5 min.
- **Accuracy benchmark on a sample:** the accuracy bar is checked on a fixed sample of 100 recordings, 50 per language, chosen deterministically from the corpus. Together with the parallel run: `e2e-bench` ≈ 9 → ≈ 2 min. `e2e-quick` keeps its 10 recordings.
- **Seam sweep out of `check`:** `chunked_transcription_it` moves from `e2e-smoke` to `e2e-seams`, which runs in `full-check` only. Expected: `check` ≈ 5.5 → ≈ 3.5 min; `full-check` unchanged by this item.
- Time estimates in `justfile` and `AGENTS.md` are updated to the measured values.

Target: `full-check` ≈ 17 → ≈ 10 min, `check` ≈ 5.5 → ≈ 3.5 min, with the same tests passing and the same bars (WER < 30 %, seam rules unchanged).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `file-transcription`: the requirement "Transcription meets the accuracy bar" is checked on a fixed, language-balanced sample of 100 recordings of the benchmark corpus instead of every recording.

## Impact

- `e2e-test/tests/steps/file_transcription.rs` (corpus selection, parallel transcription), `e2e-test/src/corpus.rs` if the sampling lives there, `e2e-test/tests/features/file_transcription.feature`.
- `justfile`: `e2e-smoke`, `e2e-seams`, `e2e-bench` and the time estimates in recipe comments.
- `AGENTS.md` § How to verify a change: time estimates.
- The app itself does not change. The local services see two concurrent clients during the benchmark; `MAX_CONCURRENT_SCENARIOS` already allows four.

## Stage, scope and acceptance bar

- **Stage:** PoC; this change does not move the stage.
- **Scope:** QA recipes and the E2E harness only.
- **Acceptance bar:** with this change, `just check` and `just full-check` both pass and are timed again under the same setup as the baseline: `full-check` ≤ 11 min, `check` ≤ 4 min.

## Out of scope

- Dropping `cargo clean` from `full-check`.
- The flaky `should_retry_when_service_drops_connection` test (filed separately).
- Build-profile tuning, merging integration-test binaries, faster models.
- Parallel runs of the seam scenarios.

## Version

No release: no app behavior changes; `app/Cargo.toml` keeps its version.
