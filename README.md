# voicepuncher

## Overview

Push-to-talk voice-to-text CLI for macOS Apple Silicon. Hold a hotkey, speak, release — cleaned text is pasted at the cursor. Fully offline: a local whisper.cpp `whisper-server` for transcription and a local llama.cpp `llama-server` (Qwen2.5) for cleanup, both running on Metal. Nothing leaves the machine — the app talks only to services on `127.0.0.1`. Optional [voice control](#voice-control) works hands-free: spoken commands start and stop a dictation and press Enter.

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

Prerequisites: Rust ≥ 1.88, [`just`](https://github.com/casey/just), Homebrew `whisper-cpp` and `llama.cpp` (`just start-model-servers` offers to install them).

```bash
git clone https://github.com/pirx/voicepuncher
cd voicepuncher
cargo build --release
```

Binary: `target/release/voicepuncher`

## Local services

voicepuncher runs no model itself. It sends each recording to a **speech recognition service** (OpenAI Audio Transcriptions API) and each raw transcript to a **text refinement service** (OpenAI Chat Completions API), both on this machine, and pastes what comes back. The supported services are whisper.cpp `whisper-server` and llama.cpp `llama-server`.

```bash
just start-model-servers   # starts whisper-server on 127.0.0.1:8081 and llama-server on 127.0.0.1:8080
just stop-model-servers    # stops both again (or one: just stop-model-servers llama-server)
```

`start-model-servers` checks that `whisper-server` and `llama-server` are installed (offers `brew install whisper-cpp` / `brew install llama.cpp`), skips a service if something already answers on its port, uses the largest `ggml-*.bin` and `*.gguf` in `~/.voicepuncher/` or `~/.voicepuncher/models/` (override with `VOICEPUNCHER_WHISPER_MODEL`, `VOICEPUNCHER_LLM_MODEL`), and waits until each model is loaded. whisper-server also needs the Silero voice activity detection model `ggml-silero-*.bin` (`just models` downloads it; override with `VOICEPUNCHER_VAD_MODEL`): it removes silences longer than 2 s, so a silent recording yields no text, while the pauses between words stay. Logs: `~/.voicepuncher/logs/whisper-server-8081.log`, `~/.voicepuncher/logs/llama-server-8080.log`. Set `VOICEPUNCHER_STT_PORT` / `VOICEPUNCHER_LLM_PORT` to use other ports; PID and log files are per port. `stop-model-servers` stops only servers that `start-model-servers` launched. A service that is already running keeps the flags it was started with: after an update that changes them, restart it (`just stop-model-servers whisper-server && just start-model-servers`).

To run the services by hand instead:

```bash
whisper-server -m ~/.voicepuncher/models/ggml-large-v3-turbo.bin \
  --host 127.0.0.1 --port 8081 --inference-path /v1/audio/transcriptions \
  -l auto -bo 1 -nf -sns -mc 0 \
  --vad -vm ~/.voicepuncher/models/ggml-silero-v5.1.2.bin -vsd 2000 -vp 500
llama-server -m ~/.voicepuncher/models/Qwen2.5-3B-Instruct-Q4_K_M.gguf \
  --host 127.0.0.1 --port 8080 -c 4096 -np 1
```

The app checks both services at startup and exits with code 2 if one is not reachable, or if the text refinement service offers several models and none matches `[refinement].model`. Only addresses on this machine are accepted (`localhost`, `127.0.0.0/8`, `[::1]`). With `--no-llm` the text refinement service is neither checked nor used. If refinement fails during a recording (service stopped, timeout, cut-off or too short answer), the raw transcript is pasted.

### Command spotting (voice control)

With [voice control](#voice-control) on, the app also needs the **command spotting service** `voicepuncher-command-spotting`, built from this repository (`command-spotting/`): sherpa-onnx keyword spotting with the English `zipformer-gigaspeech-3.3M` model and a Silero voice activity detector. The app streams the microphone to it over a loopback WebSocket while listening; it reports the spotted phrases and where speech starts and ends. `just models` downloads both models into `~/.voicepuncher/models/` (override with `VOICEPUNCHER_KWS_MODEL_DIR`, `VOICEPUNCHER_KWS_VAD_MODEL`); `just start-model-servers` builds the service and starts it on `127.0.0.1:8082` (`VOICEPUNCHER_KWS_PORT`), log `~/.voicepuncher/logs/voicepuncher-command-spotting-8082.log`. With voice control on, the app checks it at startup and exits with code 2 when it does not answer.

### Text refinement

The text refinement service **normalizes** the dictation: punctuation, capitals, misrecognized words, technical terms in their usual spelling (`докер` → `Docker`). Filler sounds (`э-э`, `эм`, `um`, `uh`) are removed by the app before the transcript is sent. Meaning and language stay as dictated; the transcript is data, so a dictated "переведи это на английский" is pasted as that sentence, not translated.

The app pastes the service's answer only when it is still the dictation: the same writing system and at least half of the transcript's Cyrillic and half of its Latin words. Otherwise — a translation, an answer to a dictated question, a rewrite — it logs `refinement answer rejected (…)` and pastes the raw transcript.

The default instruction comes with three example turns (`[refinement].examples`, each `{ transcript, normalized }`; `examples = []` sends none). A `system_prompt` set in your config replaces the default instruction — delete it to get the current one.

**Speech language:** keep `[transcription].language = "auto"` (the default) when you dictate in more than one language. A fixed language makes the speech recognition service write speech in another language as text in the fixed one: with `language = "ru"`, 29 of 40 English corpus samples came out as Russian, mostly garbled.

## Models

Download to `~/.voicepuncher/models/` (`just models` fetches a small default pair):

| Model | Used by | File | Source |
|---|---|---|---|
| Whisper | speech recognition service | `ggml-large-v3-turbo.bin` | https://huggingface.co/ggerganov/whisper.cpp |
| LLM | text refinement service | `Qwen2.5-3B-Instruct-Q4_K_M.gguf` | https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF |

## Quick start

```bash
cp app/config.example.toml ~/.voicepuncher/config.toml
just start-model-servers
./target/release/voicepuncher
```

Default hotkey is `RightOption`.

## Voice control

Hands-free dictation, off by default. Turn it on with `--voice-control` or in the config:

```toml
[voice_control]
enabled = true
```

Listening starts with the app (sound `Tink`); the hotkey no longer records, it turns **listening** off (`Bottle`) and on again. While listening the microphone stays open and five English commands take effect, each said on its own with a pause before and after it — the same words inside a sentence are ignored:

| Say | While | Effect |
|---|---|---|
| `start dictation` | listening, no dictation | starts a dictation (start sound) |
| `stop dictation` | dictating | ends it; the text is pasted without the words "stop dictation" |
| `cancel dictation` | dictating | discards it; nothing is pasted |
| `press enter` | listening, no dictation | presses Return in the focused app |
| `clear all text` | listening, no dictation | presses Cmd+A, then Delete, in the focused app |

A command said in the other state is ignored. `press enter` said right after `stop dictation` waits until the text is pasted. Pressing the hotkey during a dictation pastes it and turns listening off. `press enter` and `clear all text` play the command sound (`Morse`); the sounds are `[sounds]` settings.

**Careful with `clear all text`:** it selects everything in the focused app before deleting — meant for a chat input, it erases a whole document in an editor.

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
--voice-control           Voice control: the hotkey turns listening on and off, spoken commands drive dictation
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
language = "ru"                            language = "auto"   # "ru" turns English speech into Russian text
initial_prompt = "Docker, API"             prompt = "Docker, API"
threads = 4                                # whisper-server -t 4
chunk_threshold_secs = 15                  chunk_threshold_secs = 15
```

`beam_size`, `patience`, `temperature_inc`, `entropy_thold`, `logprob_thold`, `no_speech_thold` and `flash_attn` are now `whisper-server` launch flags. New keys: `model` (sent as-is), `temperature`, `timeout_secs` (per request, default 60). Then run `just start-model-servers`.

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

New keys: `model` (default `local`; `llama-server` accepts any name), `api_key` (sent as a bearer token when not empty), `timeout_secs` (per request, default 30). Then run `just start-model-servers`.

## Minimum press time

Recordings shorter than `min_press_ms` (default 200 ms) are silently dropped. This prevents accidental pastes from brief key touches.

## File mode (single WAV → stdout)

Feed a pre-recorded WAV instead of using the hotkey. Output goes to stdout — useful for scripting and E2E tests.

```bash
just start-model-servers
./target/release/voicepuncher --input-file recording.wav
```

WAV must be 16 kHz, mono, 16-bit PCM.

## Testing

All checks run through [`just`](https://github.com/casey/just) (`brew install just`). `just` with no arguments lists every recipe.

| Command | What it runs | Time | When |
|---|---|---|---|
| `just test` | unit + architecture + integration tests | ~1 min | after every edit |
| `just check` | fmt, clippy, `test`, release build, E2E smoke, WER accuracy on 10 samples, refinement evaluation | ~3 min | often while developing, before every commit you push |
| `just full-check` | `cargo clean`, fmt, clippy, `test`, release build, E2E smoke, chunk seams, WER accuracy on a fixed sample of 100 recordings, refinement evaluation | ~11 min | before merging a branch and before every release |

Each step also has its own recipe: `fmt`, `lint`, `build`, `e2e-smoke`, `e2e-quick` (10 samples), `e2e-seams` (the seam sweep, plus Russian and English speech under 11 recording conditions, transcribed in chunks), `e2e-bench` (100 samples, two at a time), `e2e` (smoke + seams + 100 samples + refinement evaluation), `refine-eval` (the `@refinement` scenarios: dictated commands, normalization, and a per-category score of the 41 dictations in `e2e-test/refinement/eval_set.jsonl` against the live text refinement service, next to the raw-transcript baseline; fails when a dictated command is carried out or the language changes). The E2E recipes start each local service that is not running and stop it afterwards, also when a test fails; a service that is already running — your everyday one included — is reused and left running.

**Models and corpus** are found automatically: the largest `ggml-*.bin` and `*.gguf` in `~/.voicepuncher/` or `~/.voicepuncher/models/`, and the corpus in `e2e-test/corpus/`. Override with `VOICEPUNCHER_WHISPER_MODEL`, `VOICEPUNCHER_LLM_MODEL`, `VOICEPUNCHER_BENCH_CORPUS` (env or `.env`). If they are missing, `check`, `full-check` and the E2E recipes stop with an error instead of skipping. First-time setup: `just models`, `just corpus`.

`full-check` runs `cargo clean`, so `target/release/voicepuncher` is gone until the build finishes.

### Release checklist

1. `just full-check` passes.
2. Manual run (required before merging a change to hotkey, recording, overlay, paste or permissions) — `cargo run --release -p voicepuncher`, then check:
   - [ ] Short phrase: press, speak, release → text is pasted.
   - [ ] Long dictation (> 30 s, crosses a chunk boundary) → no repeated or missing words at the seam.
   - [ ] Mixed Russian + English ("задеплой в Docker") → each language kept as spoken, terms in Latin script.
   - [ ] Same with `--no-llm` → the raw transcript is pasted.
   - [ ] `just stop-model-servers whisper-server`, then start the app → it exits naming the speech recognition service address and `just start-model-servers`.
   - [ ] Stop the speech recognition service while the app runs and record → nothing is pasted, the failure is logged; after `just start-model-servers` the next recording works.
   - [ ] `just stop-model-servers llama-server`, then start the app → it exits naming the text refinement service address and `just start-model-servers`.
   - [ ] With the text refinement service stopped, start with `--no-llm` → the raw transcript is pasted.
   - [ ] Stop the text refinement service while the app runs and record → the raw transcript is pasted, the failure is logged.
   - [ ] Overlay shows while recording and while processing, also on a second display if present.
   - [ ] A tap shorter than the minimum press time is ignored.
   - [ ] `just test-real-keyboard` passes (hotkey still registered after a paste).
   - [ ] Voice control (`cargo run --release -p voicepuncher -- --voice-control`): the app plays the listening-on sound at start; in a chat input "start dictation", a sentence, "stop dictation" → the sentence is pasted without the stop phrase; "press enter" → the message is sent; "start dictation", speech, "cancel dictation" → nothing is pasted; type text, "clear all text" → it is erased; "I will press enter later" said without pausing → nothing happens; the hotkey plays the listening-off sound. The log shows each `voice control: "…"` line.
   - [ ] `just stop-model-servers voicepuncher-command-spotting`, then start with `--voice-control` → it exits naming the command spotting service and `just start-model-servers`.
3. After the merge, on a clean checkout of `origin/main` with a bumped `version` in `app/Cargo.toml`: `just release` — tags `vX.Y.Z` and publishes a GitHub Release with the binary tarball and its SHA-256.

### Model comparison (not a release gate)

Run only when changing a model, decoding params or the prompt. Missing models and corpus are downloaded automatically. For each Whisper model the script starts its own `whisper-server` on port 8091, and for each LLM its own `llama-server` on port 8090, with the `start-model-servers` flags.

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
