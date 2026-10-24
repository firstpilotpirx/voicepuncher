## Context

The proposal's measurements: `just test` fails late (58 % of its failures are size budgets, metrics or compile errors, all found in seconds alone), one recording test holds 74 s of its 117 s, and `full-check` reruns every model stage for almost any edit under `app/`. The stage runner (`dev-tools stages`) already stops at the first failed stage before the model phase and already reuses a stage whose key (the files `stage_inputs` names for it) has a pass; `full-check` already picks between two accuracy stages by the changed paths (`dev-tools accuracy-stage`, `recognition_paths`). This change builds on those three mechanisms rather than adding new ones.

## Goals / Non-Goals

**Goals:** `just static` in about 10 s after a one-file edit, failing fast; `just test` stopping at it; an overlay-only change reusing the seams, outages and cues stages; the seams stage about a third of its length unless the chunking code changed; the recording test under 10 s.

**Non-Goals:** parallel model stages on the GPU; a nightly run; the strict clippy ratchet in the fast check; the hosted CI's runtime.

## Decisions

### 1. `just static`: four checks, one recipe, first stage of `test`

`_static` runs, under `set -euo pipefail`, in this order, each announced with `● <name>`:

1. `cargo fmt --all -- --check` (0.7 s);
2. `cargo clippy --workspace --all-targets -- -D warnings` (≈ 4 s after a one-file edit; checks test code without building a test binary, and holds the function budgets through `clippy.toml`);
3. `voicepuncher-analyzers size` — the size budgets (decision 2), ≈ 1–2 s;
4. `voicepuncher-analyzers metrics --changed origin/main` — the metrics ratchet over the changed files only (decision 3), ≈ 1 s.

`set -e` makes the first failure end the recipe, so a compile error never reaches the budget checks. `static` is a code stage of the runner (`_stages static …`), unlocked under the quiet lock like `test`, with a key and pass records like the other code stages; `rules.rs` lists it among the code stages a file feeds.

- `just static` → `_stages static`.
- `just test` → `_stages static test metrics`: the runner already returns at the first unlocked stage that fails, so the test stage never starts.
- `check` and `full-check` replace their `fmt` stage with `static` (fmt is its first step), so a size or compile problem fails them before `lint`'s long analyzers.
- `just fmt` stays for the hosted CI and by hand.

Alternative rejected: a separate recipe only — the agent would still pay a full `just test` for problems it forgot to check first; a stage inside `test` only — no way to run it alone.

### 2. The size budgets move into the analyzers

`app/tests/it/size_budgets/` (7 files, the `syn` measurer, the baseline, the verdict, the fixtures) moves to `analyzers/src/size/`, called as `voicepuncher-analyzers size [--write-baseline]` from the workspace root. The baseline stays `app/tests/size_baseline.toml` and its format is unchanged; `just size-baseline` becomes `voicepuncher-analyzers size --write-baseline`. The fixtures become the module's unit tests (`analyzers` unit tests run in `just test`). The workspace walk reuses the metrics' file walker. `analyzers/Cargo.toml` gains `syn` and `proc-macro2` at the exact versions the app's dev-dependencies already lock. The `it` binary loses its slowest-to-link module, and `size_budgets` leaves `app/tests/it/main.rs`.

The size analyzer prints one line per breach, in the form the test printed, and `✓ size: N units, none beyond app/tests/size_baseline.toml` when clean.

### 3. Metrics over the changed files

`voicepuncher-analyzers metrics --changed <base>` takes the files changed against the merge base with `<base>` (committed, uncommitted and untracked, as `dev-tools accuracy-stage` reads them; deleted files skipped), runs `rust-code-analysis-cli` on those `.rs` files only and compares only their `cyclomatic <file>::<function>` findings with `.config/analyzers/metrics.toml`: a new function over the threshold or a grown one fails; a shrunk or gone one is left to the full `metrics` stage (which still demands `just analyzers-baseline`). Cycles, `unsafe` per crate and duplicated lines need the whole workspace and stay in the full `metrics` stage of `test`. No report is printed in this mode; with no changed `.rs` file it passes at once. `--changed` with `--write-baseline` is refused.

### 4. The recording test, scaled down

`should_keep_every_sample_of_a_long_dictation_in_order_on_a_busy_machine` feeds 60 s in real time through the real 10 s ring: six ring-fulls. It keeps that ratio at a tenth of the time: a ring of 1 s and 6 s of audio, fed in real time, with the same CPU hogs and the same three assertions (nothing dropped, every sample there, in order). What it proves — the drain keeps up with real-time delivery under load for several times the ring's capacity — holds; the ring size is a parameter of `stand_in` already. Run time ≈ 6 s. The speech-recognition spec keeps its minute: the test proves the same property at scale, and its comment says so.

### 5. Stage inputs for the overlay and the menu bar

`stage_inputs` gains lines for the code no model stage's outcome depends on beyond the app starting:

```
app/src/adapters/secondary/visualfeedback/macos/*  e2e-smoke-voice,e2e-bundle
app/src/adapters/primary/menubar/macos/*           e2e-smoke-voice,e2e-bundle
app/src/adapters/primary/settings/macos/*          e2e-smoke-voice,e2e-bundle
app/src/core/domain/level_history.rs               e2e-smoke-voice,e2e-bundle
```

They feed the code stages, the voice control smoke (it starts the app, so a crash at start still fails) and the bundle; not seams, outages, cues, regression, the speech and services smoke or accuracy. The overlay's signs the outage scenarios check come from `core/application/visual_indicator.rs`, which stays unnamed and feeds every stage. Everything else stays as it is: an unnamed file feeds every stage (erring towards running). `improve-recording-pill-look`'s files would have run 2 model stages instead of 10.

### 6. The seams stage: a sample unless the chunking code changed

Two stages, as for accuracy, so that a sample pass is never reused where the matrix is required (a squash merge gives `main` the branch's exact files):

- `e2e-seams` — the whole matrix, as today;
- `e2e-seams-sample` — the sample.

`dev-tools seams-stage [base]` prints `e2e-seams — <reason>` or `e2e-seams-sample — <reason>`, with `accuracy-stage`'s rules: the matrix on `main` itself, when the changed paths cannot be read, or when a changed path matches the new `chunking_paths` table of `.config/stages.toml`; else the sample. `full-check` asks it once and prints `● seams: …` before its first stage, beside `● accuracy: …`. The two choosers share one function over a table name.

`chunking_paths`:

```
app/src/core/application/chunking.rs
app/src/core/application/chunking_seam_cases.rs
app/src/core/domain/audio_buffer.rs
app/src/adapters/secondary/transcription/*
app/src/config.rs
app/src/config/*
app/tests/it/chunked_transcription.rs
e2e-test/src/speech.rs
e2e-test/tests/features/speech_recognition.feature
e2e-test/tests/steps/speech_recognition.rs
e2e-test/tests/steps/speech_recognition/*
```

The sample:

- the seam sweep (`chunked_transcription`): lead-in 0 only — 10 of its 30 cases — when `VOICEPUNCHER_SEAM_SAMPLE=1`, which `_e2e-seams-sample` exports;
- the `@seams` scenarios: their Examples split into a block tagged `@seams_sample` and the rest; the nextest profile `e2e-seams-sample` runs `@seams and @seams_sample and not @wip`. The sample rows: difficult conditions — Russian spoken fast, Russian with high background noise, English with pauses between sentences, English through a phone-quality line (4 of 20); pauses — Russian with pauses between words, English with pauses between sentences (2 of 4); lead-ins — 0 s (1 of 3).

Estimated: 252 s → about 90 s. The sample keeps both languages, both pause kinds and a lead-in.

### 7. Fewer example rows

Only the `@regression` rows of `voice_control.feature` (72 rows, run only by `full-check`) are cut: per outline, keep one row per command and drop the rows that differ only in a spoken form the `voice_forms` integration test already checks against the command spotting service, or only in the words around a command. Target: at most 4 regression rows per outline (72 → about 30). `@smoke` rows (2–3 per command) and every `@voice_samples` row stay. `spec_mirror` and the `feature-lint` keep every requirement covered. Gain ≈ 15 s; the larger savings are decisions 5 and 6.

### Expected `full-check` (warm caches, no recognition-path change)

| Change kind | Model stages that run | ≈ time |
|---|---|---|
| overlay / menu bar | smoke-voice, bundle | 4 min |
| voice control | smoke-voice, regression, resilience, seams-sample, cues, services, quick | 12 min |
| chunking | smoke-speech, seams, quick or bench | 9 min |

## Risks / Trade-offs

- [A stage input line too narrow lets a breaking edit reuse a stale pass] → lines only for macOS drawing code no scenario reads; everything else stays unnamed and feeds every stage; `main` runs the matrix and the full set on each landing group.
- [The scaled recording test misses a drain bug that only shows after a minute] → the ratio of audio to ring is kept; a drain that falls behind falls behind in the first ring-full.
- [The seam sample misses a seam regression outside its rows] → the sample keeps each language and pause kind; any change to the chunking code runs the matrix.
- [`metrics --changed` misses a complexity rise in an unchanged file] → impossible: an unchanged file's functions do not change; cycles and duplicates stay in the full stage.

## Migration Plan

One branch; `just size-baseline` and `just analyzers-baseline` keep their names. AGENTS.md § How to verify a change and § TDD name `just static` as the step after an edit, before `just test`.

## Test mapping

All in `dev-tools/tests/features/change_workflow.feature` (slice, harness `dev-tools/tests/cucumber_workflow.rs`), steps in `dev-tools/tests/steps/analyzers.rs` and `dev-tools/tests/steps/stage_choice.rs`.

| Scenario | Test |
|---|---|
| A function over budget fails the static check without a test (change-workflow) | slice `change_workflow.feature` `A function over budget fails the static check without a test` |
| A compile error fails the static check (change-workflow) | slice `change_workflow.feature` `A compile error fails the static check` |
| A new complex function in a changed file fails the static check (change-workflow) | slice `change_workflow.feature` `A new complex function in a changed file fails the static check` |
| The test recipe stops at the static check (change-workflow) | slice `change_workflow.feature` `The test recipe stops at the static check` |
| A clean tree passes the static check in seconds (change-workflow) | slice `change_workflow.feature` `A clean tree passes the static check in seconds` |
| An overlay change skips the seams, outages and start cues (change-workflow) | slice `change_workflow.feature` `An overlay change skips the seams, outages and start cues` |
| A change to the chunking code runs the seams stage (change-workflow) | slice `change_workflow.feature` `A change to the chunking code runs the seams stage` |
| A file no stage names runs every stage (change-workflow) | slice `change_workflow.feature` `A file no stage names runs every stage` |
| A branch outside the chunking code runs the seam sample (change-workflow) | slice `change_workflow.feature` `A branch outside the chunking code runs the seam sample` |
| A branch that changes the chunking code runs the whole matrix (change-workflow) | slice `change_workflow.feature` `A branch that changes the chunking code runs the whole matrix` |
