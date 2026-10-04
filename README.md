# voicepuncher

## Overview

Push-to-talk voice-to-text CLI for macOS Apple Silicon. Hold a hotkey, speak, release — cleaned text is pasted at the cursor. Fully offline: a local whisper.cpp `whisper-server` for transcription and a local llama.cpp `llama-server` (Qwen2.5) for cleanup, both running on Metal. Nothing leaves the machine — the app talks only to services on `127.0.0.1`.

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

Prerequisites: Rust ≥ 1.88, [`just`](https://github.com/casey/just), Homebrew `whisper-cpp` and `llama.cpp` (`just start-services` offers to install them).

```bash
git clone https://github.com/pirx/voicepuncher
cd voicepuncher
cargo build --release
```

Binary: `target/release/voicepuncher`

## Local services

voicepuncher runs no model itself. It sends each recording to a **speech recognition service** (OpenAI Audio Transcriptions API) and each raw transcript to a **text refinement service** (OpenAI Chat Completions API), both on this machine, and pastes what comes back. The supported services are whisper.cpp `whisper-server` and llama.cpp `llama-server`.

```bash
just start-services   # starts whisper-server on 127.0.0.1:8081 and llama-server on 127.0.0.1:8080
just stop-services    # stops both again (or one: just stop-services llama-server)
```

`start-services` checks that `whisper-server` and `llama-server` are installed (offers `brew install whisper-cpp` / `brew install llama.cpp`), skips a service if something already answers on its port, uses the largest `ggml-*.bin` and `*.gguf` in `~/.voicepuncher/` or `~/.voicepuncher/models/` (override with `VOICEPUNCHER_WHISPER_MODEL`, `VOICEPUNCHER_LLM_MODEL`), and waits until each model is loaded. Logs: `~/.voicepuncher/logs/whisper-server-8081.log`, `~/.voicepuncher/logs/llama-server-8080.log`. Set `VOICEPUNCHER_STT_PORT` / `VOICEPUNCHER_LLM_PORT` to use other ports; PID and log files are per port. `stop-services` stops only servers that `start-services` launched.

To run the services by hand instead:

```bash
whisper-server -m ~/.voicepuncher/models/ggml-large-v3-turbo.bin \
  --host 127.0.0.1 --port 8081 --inference-path /v1/audio/transcriptions \
  -l auto -bo 1 -nf -sns -mc 0 -nt
llama-server -m ~/.voicepuncher/models/Qwen2.5-3B-Instruct-Q4_K_M.gguf \
  --host 127.0.0.1 --port 8080 -c 4096 -np 1
```

The app checks both services at startup and exits with code 2 if one is not reachable, or if the text refinement service offers several models and none matches `[refinement].model`. Only addresses on this machine are accepted (`localhost`, `127.0.0.0/8`, `[::1]`). With `--no-llm` the text refinement service is neither checked nor used. If refinement fails during a recording (service stopped, timeout, cut-off or too short answer), the raw transcript is pasted.

## Models

Download to `~/.voicepuncher/models/` (`just models` fetches a small default pair):

| Model | Used by | File | Source |
|---|---|---|---|
| Whisper | speech recognition service | `ggml-large-v3-turbo.bin` | https://huggingface.co/ggerganov/whisper.cpp |
| LLM | text refinement service | `Qwen2.5-3B-Instruct-Q4_K_M.gguf` | https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF |

## Quick start

```bash
cp app/config.example.toml ~/.voicepuncher/config.toml
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
--refinement-url <URL>    Override [refinement].base_url (text refinement service, loopback only)
--no-llm                  Skip text refinement; paste the raw transcript without contacting the service
--debug-save-wav          Save recorded WAV for debugging
-v                        Verbose logging
-vv                       Trace logging
```

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Clean shutdown (SIGTERM / SIGINT) |
| 1 | Runtime crash (hotkey listener died) |
| 2 | Startup misconfiguration (bad or outdated config, a local service not reachable, configured model not offered, permission denied) |

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

## Upgrading from the in-process LLM

The `[llm]` config section and `--llm-model` are gone (use `--refinement-url`). A config that still has `[llm]` makes the app exit with a message naming each key's replacement:

```toml
# before                                   # after
[llm]                                      [refinement]
model_path = "~/.voicepuncher/qwen.gguf"   base_url = "http://127.0.0.1:8080/v1"   # model: llama-server -m
context_size = 4096                        # llama-server -c 4096
system_prompt = "..."                      system_prompt = "..."
max_tokens = 512                           max_tokens = 512
temperature = 0.0                          temperature = 0.0
```

New keys: `model` (default `local`; `llama-server` accepts any name), `api_key` (sent as a bearer token when not empty), `timeout_secs` (per request, default 30). Then run `just start-services`.

## Minimum press time

Recordings shorter than `min_press_ms` (default 200 ms) are silently dropped. This prevents accidental pastes from brief key touches.

## File mode (single WAV → stdout)

Feed a pre-recorded WAV instead of using the hotkey. Output goes to stdout — useful for scripting and E2E tests.

```bash
just start-services
./target/release/voicepuncher --input-file recording.wav
```

WAV must be 16 kHz, mono, 16-bit PCM.

## Testing

All checks run through [`just`](https://github.com/casey/just) (`brew install just`). `just` with no arguments lists every recipe.

| Command | What it runs | Time | When |
|---|---|---|---|
| `just test` | unit + architecture + integration tests | ~1 min | after every edit |
| `just check` | fmt, clippy, `test`, release build, E2E smoke, WER accuracy on 10 samples | ~4 min | often while developing, before every commit you push |
| `just full-check` | `cargo clean`, fmt, clippy, `test`, release build, E2E smoke, WER accuracy on the whole corpus | ~30 min | before merging a branch and before every release |

Each step also has its own recipe: `fmt`, `lint`, `build`, `e2e-smoke`, `e2e-quick` (10 samples), `e2e-bench` (whole corpus), `e2e` (smoke + whole corpus). The E2E recipes start each local service that is not running and stop it afterwards, also when a test fails; a service that is already running — your everyday one included — is reused and left running.

**Models and corpus** are found automatically: the largest `ggml-*.bin` and `*.gguf` in `~/.voicepuncher/` or `~/.voicepuncher/models/`, and the corpus in `e2e-test/corpus/`. Override with `VOICEPUNCHER_WHISPER_MODEL`, `VOICEPUNCHER_LLM_MODEL`, `VOICEPUNCHER_BENCH_CORPUS` (env or `.env`). If they are missing, `check`, `full-check` and the E2E recipes stop with an error instead of skipping. First-time setup: `just models`, `just corpus`.

`full-check` runs `cargo clean`, so `target/release/voicepuncher` is gone until the build finishes.

### Release checklist

1. `just full-check` passes.
2. Manual run (required before merging a change to hotkey, recording, overlay, paste or permissions) — `cargo run --release -p voicepuncher`, then check:
   - [ ] Short phrase: press, speak, release → text is pasted.
   - [ ] Long dictation (> 30 s, crosses a chunk boundary) → no repeated or missing words at the seam.
   - [ ] Mixed Russian + English ("задеплой в Docker") → each language kept as spoken, terms in Latin script.
   - [ ] Same with `--no-llm` → the raw transcript is pasted.
   - [ ] `just stop-services whisper-server`, then start the app → it exits naming the speech recognition service address and `just start-services`.
   - [ ] Stop the speech recognition service while the app runs and record → nothing is pasted, the failure is logged; after `just start-services` the next recording works.
   - [ ] `just stop-services llama-server`, then start the app → it exits naming the text refinement service address and `just start-services`.
   - [ ] With the text refinement service stopped, start with `--no-llm` → the raw transcript is pasted.
   - [ ] Stop the text refinement service while the app runs and record → the raw transcript is pasted, the failure is logged.
   - [ ] Overlay shows while recording and while processing, also on a second display if present.
   - [ ] A tap shorter than the minimum press time is ignored.
   - [ ] `just test-real-keyboard` passes (hotkey still registered after a paste).
3. After the merge, on a clean checkout of `origin/main` with a bumped `version` in `app/Cargo.toml`: `just release` — tags `vX.Y.Z` and publishes a GitHub Release with the binary tarball and its SHA-256.

### Model comparison (not a release gate)

Run only when changing a model, decoding params or the prompt. Missing models and corpus are downloaded automatically. For each Whisper model the script starts its own `whisper-server` on port 8091, and for each LLM its own `llama-server` on port 8090, with the `start-services` flags.

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
