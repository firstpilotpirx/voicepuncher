# E2E Benchmark: Design Decisions

> Captures the architectural decisions behind honest end-to-end testing and the
> `--input-file` mode. This document is the source of truth for why things are
> structured the way they are.
>
> **Status:** the original design, implemented. The file names below are the
> ones that exist today; `docs/project-map.md` maps the whole E2E crate, and the
> E2E scenarios are Gherkin features run by `e2e-test/tests/cucumber_e2e.rs`.

---

## Problem

voicepuncher records audio from the microphone. A true E2E test must drive
the **built binary** through its real entry point (CLI) without injecting into
its internals. But the microphone cannot be scripted in CI.

Three options considered:

| Option | Description | Verdict |
|---|---|---|
| A — Integration test inside `app/` | Mock the `AudioRecorder` port, inject a pre-loaded buffer | Not E2E — skips the binary boundary, skips CLI parsing, skips real adapter wiring |
| B — E2E in `e2e-test/`, pipe audio via virtual mic (e.g. BlackHole) | Fork the real binary, feed audio via a virtual audio device | Requires per-machine audio setup, not portable, fragile in CI |
| C — E2E in `e2e-test/`, add `--input-file` to the binary | Fork the real binary with `--input-file <path>`, capture stdout | **Chosen** — honest black-box, no special hardware, CI-friendly |

Option C is the only approach that satisfies all three constraints:
1. Drives the **built binary** (not source), through the real CLI.
2. Requires **no microphone** or virtual audio device.
3. Produces **measurable output** (stdout) comparable to a ground-truth transcript.

---

## `--input-file` Mode Design

### What it does

When `--input-file <path>` is passed:

- Skips the hotkey listener (no `rdev` thread).
- Reads the audio file from disk instead of the microphone.
- Runs the pipeline exactly once: transcribe → refine → output.
- Writes the result text to **stdout** (instead of pasting via accessibility API).
- Exits with code 0 on success, non-zero on failure.

The binary behaves like a standard Unix filter: audio in via flag, text out via stdout.

### CLI change

```
voicepuncher --input-file <path> [--config <path>] [--whisper-model <path>] [--llm-model <path>]
```

`--input-file` is mutually exclusive with normal (hotkey) mode.
Nothing is asked in a terminal; a missing configuration file is written with the defaults.

### Supported audio formats

whisper.cpp internally accepts: WAV, FLAC, MP3, OGG (via the `whisper_full` C API).
The benchmark datasets provide WAV or FLAC at 16 kHz — both work without resampling.
FLAC is preferred (lossless, ~3× smaller than WAV).

### Output format

Plain text on stdout, one result per invocation:

```
Hello world, this is a test sentence.
```

No log lines on stdout in file mode — logs stay on stderr (controlled by `--log-level` or
`[logging] level` in config). This keeps stdout parseable by the E2E test runner.

### Adapter changes

| Component | Normal mode | File mode |
|---|---|---|
| `AudioRecorder` port | `CpalAudioRecorder` (mic) | `FileAudioReader` (new secondary adapter) |
| `TextPaster` port | `MacOsTextPaster` (accessibility API) | `StdoutTextPaster` (new secondary adapter) |
| Hotkey listener | `RdevHotkeyListener` | not started |
| Pipeline | hold/toggle loop | single-shot run, then exit |

`FileAudioReader` and `StdoutTextPaster` are thin adapters, no business logic.
`main.rs` selects them when `--input-file` is present.

### Architecture compliance

Both adapters live under `app/src/adapters/secondary/`:
- `recording/filesystem/` — `FileSystemAudioRecorder` implements the `AudioRecorder` port, reads the file and returns `AudioBuffer16k`
- `textpasting/stdio/` — `StdioTextPaster` implements the `TextPaster` port, writes to stdout

Each has a matching integration test module in `app/tests/it/` (R4 architecture rule):
- `app/tests/it/recording_filesystem.rs`
- `app/tests/it/textpasting_stdio.rs`

---

## E2E Test Location

```
voicepuncher/
├── app/             ← binary + library; no E2E here
├── e2e-test/        ← separate Rust crate, drives the built binary
│   ├── Cargo.toml
│   ├── tests/
│   │   ├── cucumber_e2e.rs          ← the Gherkin harness
│   │   ├── features/                ← one .feature per capability (@smoke, @accuracy, …)
│   │   └── steps/                   ← step definitions per capability
│   └── scripts/                     ← bench.py (model matrix), download_corpus.py
└── docs/
```

**Why a separate crate, not `app/tests/`:**

| Constraint | `app/tests/` | `e2e-test/` |
|---|---|---|
| Must NOT import backend modules | Hard to enforce in the same crate | Enforced by crate boundary |
| Drives built binary, not source | Integration tests link the library | Separate crate has no `app` dependency |
| Independent build / CI stage | Rebuilds with every `app` change | Only rebuilds when artifact changes |
| Path to binary | Compile-time linked | Passed via env var `VOICEPUNCHER_BIN` |

This matches the repository-layout rule: E2E tests live in a top-level sibling root and
communicate with the SUT **only through its CLI/HTTP/stdio boundary**.

---

## E2E Test Runner Design

### How a single test works

```
1. Resolve binary path from env: VOICEPUNCHER_BIN or target/release/voicepuncher
2. Resolve model paths from env: VOICEPUNCHER_WHISPER_MODEL, VOICEPUNCHER_LLM_MODEL
3. Fork the binary:
     voicepuncher --input-file <audio.wav> \
                  --whisper-model <whisper.bin> \
                  --llm-model <llm.gguf> \
                  --config /dev/null   (or a minimal test config)
4. Capture stdout → hypothesis string
5. Load ground-truth reference from the dataset manifest
6. Compute WER and CER (see metrics section)
7. Assert WER < threshold  (or just record — regression mode)
```

### N×N matrix runner

For benchmark (not regression) mode, the test iterates cartesian product of models:

```
for whisper_model in [large-v3-turbo, small, medium]:
    for llm_model in [Qwen2.5-1.5B, Qwen2.5-3B, none]:
        for sample in corpus:
            run binary → collect WER/CER
        record mean WER/CER per (whisper, llm, dataset, language)

emit results/YYYY-MM-DD_HHMM/summary.md
```

The "none" LLM row is the control: raw whisper output without correction.
This isolates whisper contribution from LLM contribution.

### Env vars (no hardcoded paths)

| Env var | Default | Purpose |
|---|---|---|
| `VOICEPUNCHER_BIN` | `../app/target/release/voicepuncher` | Path to binary |
| `VOICEPUNCHER_WHISPER_MODEL` | — | Whisper model for smoke/regression tests |
| `VOICEPUNCHER_LLM_MODEL` | — | LLM model for smoke/regression tests |
| `VOICEPUNCHER_BENCH_CORPUS` | `./corpus` | Root of downloaded dataset samples |
| `VOICEPUNCHER_BENCH_MODELS_DIR` | `~/.voicepuncher` | Root for benchmark model files |

Tests that require models are skipped (not failed) when the env vars are absent —
same pattern as `app/tests/it/transcription_openai.rs` and `app/tests/it/textrefinement_openai.rs`.

### WER/CER computation

No mature WER crate exists in Rust (2025). Two options:

**Option A — shell out to `jiwer` (Python)**
```rust
let output = std::process::Command::new("python3")
    .args(["-c", &format!(
        "import jiwer; print(jiwer.wer({:?}, {:?}))", reference, hypothesis
    )])
    .output()?;
```
Simple, correct, requires Python + jiwer in the benchmark environment.

**Option B — inline Levenshtein (Rust)**
```rust
// word-level edit distance / total words in reference
fn wer(reference: &str, hypothesis: &str) -> f64 { ... }
```
No external dependency, but requires implementing the full WER algorithm
(substitution / deletion / insertion per the standard definition).

Recommendation: **Option B** for portability (no Python dependency in CI).
Add `edit-distance` or `strsim` crate and implement the WER formula on word slices.
Keep normalization (lowercase, strip punctuation, `ё→е`) in Rust.

---

## Corpus Structure

Downloaded samples (from `docs/benchmark-datasets.md`) are stored as:

```
e2e-test/corpus/
├── manifest.jsonl          ← one JSON object per sample
└── audio/
    ├── fleurs_ru_001.flac
    ├── fleurs_en_001.flac
    ├── librispeech_001.flac
    └── golos_ff_001.wav
```

Manifest format (one line per sample):

```json
{"id": "fleurs_ru_001", "path": "audio/fleurs_ru_001.flac", "reference": "это тестовое предложение", "lang": "ru", "duration_s": 7.4, "source": "fleurs"}
```

A download script (`e2e-test/scripts/download_corpus.py`) fetches the samples from
HuggingFace and writes the manifest. It is run once, not in CI (corpus is cached locally).

---

## CI Integration

Two separate CI jobs:

| Job | Trigger | What it runs |
|---|---|---|
| `build-and-unit` | every push | `cargo test` inside `app/` (unit + integration + arch) |
| `e2e-bench` | manual / nightly | build release binary → run `e2e-test/` with model env vars |

The E2E job requires models and corpus — these are too large for standard CI runners.
Run manually on a developer machine or on a self-hosted runner with models pre-cached.

---

## Summary of Changes Needed

| What | Where | Status |
|---|---|---|
| `--input-file` CLI flag | `app/src/adapters/primary/cli/` | done |
| `FileSystemAudioRecorder` secondary adapter | `app/src/adapters/secondary/recording/filesystem/` | done |
| `StdioTextPaster` secondary adapter | `app/src/adapters/secondary/textpasting/stdio/` | done |
| Integration tests for the adapters | `app/tests/it/recording_filesystem.rs`, `app/tests/it/textpasting_stdio.rs` | done |
| Single-shot pipeline path in `main.rs` | `app/src/main.rs` | done |
| `e2e-test/` crate scaffold | repo root | done |
| Smoke scenarios | `e2e-test/tests/features/` (`@smoke`) | done |
| Accuracy scenarios (WER over the corpus) | `e2e-test/tests/features/` (`@accuracy`) | done |
| Corpus download script | `e2e-test/scripts/download_corpus.py` | done |
