## Why

Measured on the changes done one after another in one session (fix-permissions-and-login-item, mirror-specs-in-features, tame-confirming-special-cases, improve-recording-pill-look), with nothing running beside them: `just test` took about 30 % of the time, `full-check` 21 %, `just test-one` 3 %, the agent's own work about 47 %, lock waits 0. The user finds this far too slow.

The stage journal of the last 7 days (1 500 runs) says why:

1. **`just test` is the agent's inner loop and fails late.** Median 1 min 52 s; 342 of its runs failed. Of those, 85 (25 %) failed on the size budgets, 47 (14 %) on the metrics ratchet and about 65 (19 %) on compile errors — together about 58 % — each failure costing another full run. Measured alone after a one-file edit, these checks take seconds: formatting 0.7 s, clippy 4 s, the size budgets 2 s (13 s with building the `it` binary it lives in), metrics 9 s. They run only inside the whole suite.
2. **One slow test sets the length of `just test`.** `recording_cpal::synthetic::should_keep_every_sample_of_a_long_dictation_in_order_on_a_busy_machine` runs 74 s of the suite's 117 s; the next slowest, `docs_check`, 25 s.
3. **`full-check` takes about 17 min per change** on the machine lock, one change at a time. Medians: chunk seams 5 min 24 s, the 100-recording accuracy sample 3 min 45 s (only when the recognition path changed), voice control smoke 2 min 30 s, outages 2 min 35 s, start cues 2 min 11 s, services smoke 1 min 39 s, bundle about 1 min, the rest about 2 min. Almost any edit under `app/` reruns every one of them: the overlay-only change `improve-recording-pill-look` ran the seams, outages and cues stages, which its code cannot affect.

## What Changes

- **A fast static check, about 10 s:** formatting, clippy (default lints), the size budgets and the metrics ratchet over the changed files only, stopping at the first failure. It is a recipe of its own (`just static`), which the agent runs after an edit instead of `just test`, and the first stage of `just test`, which then stops before building or running a single test. The size budgets leave the `it` test binary for a standalone analyzer, as the metrics already are.
- **The slow busy-machine recording test is shortened** to a few seconds while still proving that a long dictation keeps every sample in order under load; it stays in `just test`.
- **`full-check` runs only the stages an edit can affect:** each E2E stage names the files that feed it, so an overlay change no longer runs the chunk seams, outages or start cues.
- **The chunk seams stage is cut:** a fixed sample of the seam matrix on every run; the whole matrix only when the chunking code changed.
- **Fewer example rows in the scenarios:** voice control and other E2E scenarios keep the rows that each prove something different, and drop those that only repeat a unit test; every spec requirement keeps at least one covering scenario.

Current stage: PoC. Target stage: PoC.
Scope: the `justfile` recipes and stages (`static`, `test`, `check`, `full-check`), the stage inputs in `.config/stages.toml`, the analyzers (`analyzers/`), the slow recording test, the E2E scenario sets, AGENTS.md § How to verify a change and § TDD.
Acceptance bar: `just static` runs in about 10 s after a one-file edit and fails on a size-budget, metrics, formatting or compile problem without running a test; `just test` stops at that stage on such a problem and otherwise runs in about 45 s; an overlay-only change runs none of the seams, outages and cues stages in `full-check`; a typical change's `full-check` takes a few minutes (target under 6 min); every spec requirement keeps a covering test (`spec_mirror` green).
Out of scope: the app's behavior; the hosted CI's runtime; the strict clippy ratchet (stays in `just lint`); running the model stages in parallel on the GPU; a nightly run on `main`.

## Capabilities

### Modified Capabilities

- `change-workflow`: the fast static check and its place in `just test`; the size budgets checked by an analyzer instead of a test; `full-check` running only the stages an edit feeds; the reduced seams stage.

## Impact

- `justfile`, `dev-tools/` (stage runner), `.config/stages.toml`, `.config/nextest.toml`
- `analyzers/` (size budgets, metrics over changed files), `app/tests/it/size_budgets/`
- `app/tests/it/recording_cpal.rs`
- `e2e-test/tests/features/*.feature`, the seams harness and the E2E subsets (`@smoke`, `@regression`, `@seams`)
- `AGENTS.md`

## Open points

- The exact feeding files of each E2E stage — proposed in design from the crates and modules each stage exercises, erring towards running a stage.
- The size of the seam sample and which paths count as the chunking code — proposed in design.
- Which example rows to drop — proposed in design, one table per feature file.
- How short the busy-machine recording can get while still loading the machine — proposed in design (target under 10 s).

## Release

SemVer: no release — no behavior change in the app.
Target version: 0.38.4 (unchanged).
