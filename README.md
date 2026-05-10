# voicepuncher

## Overview

Push-to-talk voice-to-text CLI for macOS Apple Silicon. Hold a hotkey, speak, release — cleaned text is pasted at the cursor. Fully offline: whisper.cpp for transcription, Qwen2.5 for cleanup, both running locally via Metal.

## Hardware requirements

- Apple Silicon Mac (Metal required — no Intel support)
- 16 GB RAM minimum (both models load resident, ~3 GB working set combined)

## macOS permissions

Three TCC permissions are required. Grant each to the terminal app you launch voicepuncher from (e.g. iTerm2, Terminal, Warp). The app name appears in the error message on first run.

| Permission | Location |
|---|---|
| Microphone | System Settings → Privacy & Security → Microphone |
| Accessibility | System Settings → Privacy & Security → Accessibility |
| Input Monitoring | System Settings → Privacy & Security → Input Monitoring |

## Install

Prerequisites: Rust ≥ 1.88, CMake.

```bash
git clone https://github.com/pirx/voicepuncher
cd voicepuncher/app
cargo build --release
```

Binary: `target/release/voicepuncher`

## Models

Download to `~/.voicepuncher/models/`:

| Model | File | Source |
|---|---|---|
| Whisper | `ggml-large-v3-turbo-q5_0.bin` | https://huggingface.co/ggerganov/whisper.cpp |
| LLM | `Qwen2.5-3B-Instruct-Q4_K_M.gguf` | https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF |

## Quick start

```bash
cp app/config.example.toml ~/.voicepuncher/config.toml
# edit model paths in ~/.voicepuncher/config.toml
./target/release/voicepuncher
```

Default hotkey is `RightOption`.

## Usage example

Hold `RightOption`, say "hello world uh I think the API is fine", release.

Result pasted at cursor: `Hello world, I think the API is fine.`

## CLI options

```
--config <PATH>           Config file path  [default: ~/.voicepuncher/config.toml]
--setup                   Re-run the setup wizard even if config already exists
--hotkey <KEY>            Override [hotkey].key
--whisper-model <PATH>    Override [whisper].model_path
--llm-model <PATH>        Override [llm].model_path
--no-llm                  Skip LLM post-processing; paste raw Whisper output
--debug-save-wav          Save recorded WAV for debugging
-v                        Verbose logging
-vv                       Trace logging
```

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Clean shutdown (SIGTERM / SIGINT) |
| 1 | Runtime crash (hotkey listener died) |
| 2 | Startup misconfiguration (bad config, missing model, permission denied) |

## Minimum press time

Recordings shorter than `min_press_ms` (default 200 ms) are silently dropped. This prevents accidental pastes from brief key touches.

## File mode (single WAV → stdout)

Feed a pre-recorded WAV instead of using the hotkey. Output goes to stdout — useful for scripting and E2E tests.

```bash
./target/release/voicepuncher \
  --input-file recording.wav \
  --whisper-model ~/.voicepuncher/models/ggml-large-v3-turbo-q5_0.bin \
  --llm-model    ~/.voicepuncher/models/Qwen2.5-3B-Instruct-Q4_K_M.gguf
```

WAV must be 16 kHz, mono, 16-bit PCM.

## End-to-end tests

Requires [`just`](https://github.com/casey/just): `brew install just`.

### Benchmark commands

Two modes:

| Command | Samples | When to use |
|---|---|---|
| `just bench` | all (100+) | full accuracy measurement, before tagging a release |
| `just bench-smoke` | 10 | quick sanity check after each change (~2 min) |

Both download models and corpus automatically on first run.

Example output:

```
+--------------------+---------------+--------+--------+----------+
| Whisper            | LLM           |    WER |    CER |  Samples |
+--------------------+---------------+--------+--------+----------+
| large-v3-turbo-q5  | Qwen2.5-3B-Q4 |  32.1% |  28.0% | 100/100  |
| small              | Qwen2.5-3B-Q4 |  53.0% |  48.1% | 100/100  |
+--------------------+---------------+--------+--------+----------+
```

To add a new model to the matrix, add an entry to `WHISPER_MODELS` or `LLM_MODELS` in `e2e-test/scripts/bench.py`.

Models are stored in `~/.voicepuncher/models/` — shared with the main app, no re-download needed.

### Other recipes

| Recipe | Description |
|---|---|
| `just build` | Build the release binary |
| `just corpus` | Download benchmark corpus only (FLEURS ru/en + LibriSpeech) |
| `just e2e-smoke` | Smoke test — models load, pipeline runs |
| `just e2e-bench` | Rust WER test against corpus (single model combo via env) |
| `just e2e` | Smoke + Rust benchmark |
| `just bench` | Full benchmark — all model combos on full corpus |
| `just bench-smoke` | Quick benchmark — 10 samples, ~2 min |
