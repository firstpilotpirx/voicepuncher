# Design

## Context

The baseline timing is in proposal.md § Why. Here are the parts of the code that shape the approach:

- The `@accuracy` scenario (`e2e-test/tests/features/file_transcription.feature`) loads the whole corpus (300 recordings). It uses `select_balanced` only when `VOICEPUNCHER_BENCH_LIMIT` is set: `e2e-quick` sets it to 10, while `e2e-bench` leaves it unset. The scenario then transcribes the recordings one by one, starting the app binary once per recording.
- The two services process one request at a time: `llama-server` runs with `-np 1`, and `whisper-server` handles requests one after another. A probe confirmed this: 4 requests to `whisper-server` took 3 s whether sent one after another or all at once. A recording costs ≈ 0.75 s of speech recognition plus ≈ 0.9 s of refinement, and these two never overlap today.
- `e2e-smoke` runs `chunked_transcription_it`, the seam sweep of the `speech-recognition` scenario "Chunk seams fall on different words", which takes 134 s of 209 s. `e2e-seams` runs only the `@seams` `.feature` scenarios.

## Goals / Non-Goals

**Goals:**
- Overlap speech recognition and refinement across recordings.
- Make the sample size part of the scenario, not of the recipe.
- Keep every bar as it is: WER < 30 %, seam rules.

**Non-Goals:**
- More than two concurrent recordings.
- Changing the corpus files or the manifest.

## Decisions

1. **Sample size lives in the scenario.** The `Given` step reads the size from the scenario text ("a fixed sample of 100 recordings …") and picks the recordings with the existing `select_balanced`. That function is deterministic for a given manifest and takes the languages in turn, so it gives 50 + 50. `VOICEPUNCHER_BENCH_LIMIT` stays as a cap for the `e2e-quick` shortcut (10 recordings): when it is set and smaller, it wins.
   *Alternative:* set `VOICEPUNCHER_BENCH_LIMIT=100` only in `e2e-bench`. Rejected because the scenario would then still say "every recording of the corpus" while the recipe silently samples, so the spec and the test would disagree.

2. **Two recordings at a time, results in corpus order.** A generic helper in `e2e-test/src/` maps the items through a closure on at most N threads (`std::thread::scope`) and returns the results in input order. The `When` step calls it with N = 2. With N = 2, one recording is in speech recognition while the other is in refinement. N > 2 would only queue up in the services, and too many concurrent clients make `whisper-server` reset connections, as the comment on `MAX_CONCURRENT_SCENARIOS` warns.
   *Alternatives:* async (would need a runtime in the step code; threads are enough for two child processes). A shell `xargs -P` (would lose the per-recording WER table and the Gherkin step).

3. **Seam sweep moves to `e2e-seams`.** The `cargo test … --test chunked_transcription_it` line moves from `e2e-smoke` to `e2e-seams`. `e2e-seams` then needs `VOICEPUNCHER_TRANSCRIPTION_URL` only, which it already exports. `full-check` keeps both runs; `check` loses the sweep. The smoke adapter tests (`transcription_openai_it`, `textrefinement_openai_it`) stay in `e2e-smoke`.

4. **Time estimates.** The recipe comments in `justfile` and the table in `AGENTS.md` § How to verify a change get the values timed after the change, using the same setup as the baseline (M1 Max, large-v3-turbo, services already running, `full-check` from a clean build).

## Test levels

| Spec scenario | Level |
|---|---|
| file-transcription: "Benchmark corpus is transcribed within the accuracy bar" | E2E `.feature` (`@accuracy`, `e2e-test/tests/features/file_transcription.feature`) |
| The concurrency helper (order preserved, at most N at a time) | unit (`#[cfg(test)]` in `e2e-test/src/`) |
| speech-recognition: "Chunk seams fall on different words" (unchanged, only moved between recipes) | adapter integration against the real service (`chunked_transcription_it`) |

## Risks / Trade-offs

- [A sample of 100 detects a WER regression less precisely than 300] → mean WER is 6.5 % against a 30 % bar, and the standard error at n = 100 is well under 1 point. For a model comparison, `just bench` still uses the whole corpus.
- [A seam regression is now caught at `full-check`, not at `check`] → `full-check` runs before every merge, so no regression can reach `main`. Changes to chunking already list `e2e-seams` among their checks.
- [Two concurrent clients hit the user's live services] → this stays within the existing `MAX_CONCURRENT_SCENARIOS = 4` budget.
