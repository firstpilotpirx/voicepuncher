# voicepuncher

## Overview

Push-to-talk voice-to-text CLI for macOS Apple Silicon. Hold a hotkey, speak, release — cleaned text is pasted at the cursor. Fully offline: a local whisper.cpp `whisper-server` for transcription, Qwen2.5 for cleanup, both running locally via Metal. Nothing leaves the machine — the app talks only to services on `127.0.0.1`.

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

Prerequisites: Rust ≥ 1.88, CMake, [`just`](https://github.com/casey/just), Homebrew `whisper-cpp` (`just start-services` offers to install it).

```bash
git clone https://github.com/pirx/voicepuncher
cd voicepuncher
cargo build --release
```

Binary: `target/release/voicepuncher`

## Speech recognition service

voicepuncher does not run Whisper itself. It sends each recording to a **speech recognition service** on this machine over the OpenAI Audio Transcriptions API and pastes what comes back. The supported service is whisper.cpp `whisper-server`.

```bash
just start-services   # starts whisper-server on 127.0.0.1:8081 with your Whisper model
just stop-services    # stops it again
```

`start-services` checks that `whisper-server` is installed (offers `brew install whisper-cpp`), skips the start if something already answers on port 8081, uses the largest `ggml-*.bin` in `~/.voicepuncher/` or `~/.voicepuncher/models/` (override with `VOICEPUNCHER_WHISPER_MODEL`), and waits until the model is loaded. Log: `~/.voicepuncher/logs/whisper-server.log`. `stop-services` stops only a server that `start-services` launched.

To run the service by hand instead:

```bash
whisper-server -m ~/.voicepuncher/models/ggml-large-v3-turbo.bin \
  --host 127.0.0.1 --port 8081 --inference-path /v1/audio/transcriptions \
  -l auto -bo 1 -nf -sns -mc 0 -nt
```

The app checks the service at startup and exits with code 2 if it is not reachable. Only addresses on this machine are accepted (`localhost`, `127.0.0.0/8`, `[::1]`).

## Models

Download to `~/.voicepuncher/models/` (`just models` fetches a small default pair):

| Model | Used by | File | Source |
|---|---|---|---|
| Whisper | speech recognition service | `ggml-large-v3-turbo.bin` | https://huggingface.co/ggerganov/whisper.cpp |
| LLM | voicepuncher | `Qwen2.5-3B-Instruct-Q4_K_M.gguf` | https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF |

## Quick start

```bash
cp app/config.example.toml ~/.voicepuncher/config.toml
# edit the LLM model path in ~/.voicepuncher/config.toml
just start-services
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
--transcription-url <URL> Override [transcription].base_url (speech recognition service, loopback only)
--llm-model <PATH>        Override [llm].model_path
--no-llm                  Skip LLM post-processing; paste the raw transcript
--debug-save-wav          Save recorded WAV for debugging
-v                        Verbose logging
-vv                       Trace logging
```

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Clean shutdown (SIGTERM / SIGINT) |
| 1 | Runtime crash (hotkey listener died) |
| 2 | Startup misconfiguration (bad or outdated config, speech recognition service not reachable, missing model, permission denied) |

## Upgrading from the in-process Whisper engine

The `[whisper]` config section and `--whisper-model` are gone. A config that still has `[whisper]` makes the app exit with a message naming each key's replacement. Rename the section and move the decoder settings to the service:

```toml
# before                                   # after
[whisper]                                  [transcription]
model_path = "~/.voicepuncher/ggml.bin"    base_url = "http://127.0.0.1:8081/v1"   # model: whisper-server -m
language = "ru"                            language = "ru"
initial_prompt = "Docker, API"             prompt = "Docker, API"
threads = 4                                # whisper-server -t 4
chunk_threshold_secs = 15                  chunk_threshold_secs = 15
```

`beam_size`, `patience`, `temperature_inc`, `entropy_thold`, `logprob_thold`, `no_speech_thold` and `flash_attn` are now `whisper-server` launch flags. New keys: `model` (sent as-is), `temperature`, `timeout_secs` (per request, default 60). Then run `just start-services`.

## Minimum press time

Recordings shorter than `min_press_ms` (default 200 ms) are silently dropped. This prevents accidental pastes from brief key touches.

## File mode (single WAV → stdout)

Feed a pre-recorded WAV instead of using the hotkey. Output goes to stdout — useful for scripting and E2E tests.

```bash
just start-services
./target/release/voicepuncher \
  --input-file recording.wav \
  --llm-model ~/.voicepuncher/models/Qwen2.5-3B-Instruct-Q4_K_M.gguf
```

WAV must be 16 kHz, mono, 16-bit PCM.

## Testing

All checks run through [`just`](https://github.com/casey/just) (`brew install just`). `just` with no arguments lists every recipe.

| Command | What it runs | Time | When |
|---|---|---|---|
| `just test` | unit + architecture + integration tests | ~1 min | after every edit |
| `just check` | fmt, clippy, `test`, release build, E2E smoke, WER accuracy on 10 samples | ~4 min | often while developing, before every commit you push |
| `just full-check` | `cargo clean`, fmt, clippy, `test`, release build, E2E smoke, WER accuracy on the whole corpus | ~30 min | before merging a branch and before every release |

Each step also has its own recipe: `fmt`, `lint`, `build`, `e2e-smoke`, `e2e-quick` (10 samples), `e2e-bench` (whole corpus), `e2e` (smoke + whole corpus). The E2E recipes run `start-services` before the tests and `stop-services` after them, also when a test fails; a service you started yourself is reused and left running.

**Models and corpus** are found automatically: the largest `ggml-*.bin` and `*.gguf` in `~/.voicepuncher/` or `~/.voicepuncher/models/`, and the corpus in `e2e-test/corpus/`. Override with `VOICEPUNCHER_WHISPER_MODEL`, `VOICEPUNCHER_LLM_MODEL`, `VOICEPUNCHER_BENCH_CORPUS` (env or `.env`). If they are missing, `check`, `full-check` and the E2E recipes stop with an error instead of skipping. First-time setup: `just models`, `just corpus`.

`full-check` runs `cargo clean`, so `target/release/voicepuncher` is gone until the build finishes.

### Release checklist

1. `just full-check` passes.
2. Manual run (required before merging a change to hotkey, recording, overlay, paste or permissions) — `cargo run --release -p voicepuncher`, then check:
   - [ ] Short phrase: press, speak, release → text is pasted.
   - [ ] Long dictation (> 30 s, crosses a chunk boundary) → no repeated or missing words at the seam.
   - [ ] Mixed Russian + English ("задеплой в Docker") → each language kept as spoken, terms in Latin script.
   - [ ] Same with `--no-llm` → the raw transcript is pasted.
   - [ ] `just stop-services`, then start the app → it exits naming the service address and `just start-services`.
   - [ ] Stop the service while the app runs and record → nothing is pasted, the failure is logged; after `just start-services` the next recording works.
   - [ ] Overlay shows while recording and while processing, also on a second display if present.
   - [ ] A tap shorter than the minimum press time is ignored.
   - [ ] `just test-real-keyboard` passes (hotkey still registered after a paste).
3. After the merge, on a clean checkout of `origin/main` with a bumped `version` in `app/Cargo.toml`: `just release` — tags `vX.Y.Z` and publishes a GitHub Release with the binary tarball and its SHA-256.

### Model comparison (not a release gate)

Run only when changing a model, decoding params or the prompt. Missing models and corpus are downloaded automatically. For each Whisper model the script starts its own `whisper-server` on port 8091 with the `start-services` decoding flags.

| Command | Samples | Time |
|---|---|---|
| `just bench` | full corpus | long |
| `just bench-smoke` | 10 | ~2 min |

Example output:

```
+--------------------+---------------+--------+--------+----------+
| Whisper            | LLM           |    WER |    CER |  Samples |
+--------------------+---------------+--------+--------+----------+
| large-v3-turbo-q5  | Qwen2.5-3B-Q4 |  32.1% |  28.0% | 100/100  |
| small              | Qwen2.5-3B-Q4 |  53.0% |  48.1% | 100/100  |
+--------------------+---------------+--------+--------+----------+
```

To add a model to the matrix, add an entry to `WHISPER_MODELS` or `LLM_MODELS` in `e2e-test/scripts/bench.py`.
