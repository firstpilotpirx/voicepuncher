# Tasks

## 1. Concurrency helper

- [x] 1.1 Red: add `e2e-test/src/concurrency.rs` with unit tests `should_return_results_in_input_order` and `should_run_at_most_n_items_at_a_time` against a `map_concurrently` stub that is `todo!()`; `cargo test -p voicepuncher-e2e-test --lib concurrency` fails on the stub.
- [x] 1.2 Green: implement `map_concurrently(items, max_concurrent, f)` with `std::thread::scope`; the tests from 1.1 pass and `just test` is green.
- [x] 1.3 `just check` passes; commit `test(e2e): concurrent map helper for the accuracy benchmark`.

## 2. Accuracy scenario on a fixed sample, two recordings at a time

- [x] 2.1 Red: change the `@accuracy` scenario in `e2e-test/tests/features/file_transcription.feature` to the spec wording ("a fixed sample of 100 recordings of the benchmark corpus, half Russian and half English, with reference transcripts" / "transcribes every recording of the sample with refinement"); `just e2e-quick` fails on the undefined steps.
- [x] 2.2 Green: in `e2e-test/tests/steps/file_transcription.rs`, the `Given` step takes the sample size from the text and applies `select_balanced`, capped by `VOICEPUNCHER_BENCH_LIMIT` when that is set and smaller; it fails if the sample is not half per language. The `When` step transcribes through `map_concurrently` with 2 at a time and prints the WER table in corpus order. `just e2e-quick` passes (10 recordings) and `just e2e-bench` reports `over 100 recordings`.
- [x] 2.3 `justfile`: the `e2e-bench` comment says 100 recordings, 50 per language. `just check` passes; commit `test(e2e): accuracy benchmark on a 100-recording sample, two at a time`.

## 3. Seam sweep out of `check`

- [x] 3.1 `justfile`: move `--test chunked_transcription_it` from `e2e-smoke` to `e2e-seams`; `just e2e-smoke` no longer runs it and `just e2e-seams` runs it and passes.
- [x] 3.2 `just check` passes; commit `chore(just): seam sweep runs in e2e-seams, not in e2e-smoke`.

## 4. Timing and docs

- [x] 4.1 Time `just check` and `just full-check` under the baseline setup (proposal.md § Why); `full-check` ≤ 11 min and `check` ≤ 4 min (proposal acceptance bar).
- [x] 4.2 Update the times in the `justfile` recipe comments (`check`, `full-check`, `e2e`, `e2e-smoke`, `e2e-seams`, `e2e-bench`) and in `AGENTS.md` § How to verify a change to the values from 4.1; commit `docs: measured QA times`.

## 5. Version and manual checks

- [x] 5.1 No version bump: the proposal says no release; `app/Cargo.toml` keeps its version.
- [x] 5.2 Manual checks by the user: none (no hotkey, recording, overlay, paste or permission change).
