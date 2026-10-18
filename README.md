# voicepuncher

## Overview

Push-to-talk voice-to-text CLI for macOS Apple Silicon. Hold a hotkey, speak, release — cleaned text is pasted at the cursor. Fully offline: local whisper.cpp `whisper-server`s for transcription (large-v3-turbo for dictation, small for confirming voice commands) and a local llama.cpp `llama-server` (Qwen3-4B-Instruct-2507) for cleanup, all running on Metal. Nothing leaves the machine — the app talks only to services on `127.0.0.1`. Optional [voice control](#voice-control) works hands-free: spoken commands start and stop a dictation and press Enter.

## Hardware requirements

- Apple Silicon Mac (Metal required — no Intel support)
- 16 GB RAM minimum (all models load resident, ~6 GB working set combined: six whisper-server copies ≈ 4.3 GB, text refinement ≈ 1 GB)

## macOS permissions

Three TCC permissions are required. Grant each to the terminal app you launch voicepuncher from (e.g. iTerm2, Terminal, Warp). The app name appears in the error message on first run.

| Permission | Location |
|---|---|
| Microphone | System Settings → Privacy & Security → Microphone |
| Accessibility | System Settings → Privacy & Security → Accessibility |
| Input Monitoring | System Settings → Privacy & Security → Input Monitoring |

## Install

Prerequisites: Rust ≥ 1.88, [`just`](https://github.com/casey/just), Homebrew `whisper-cpp`, `llama.cpp` and `haproxy` (`just start-model-servers` offers to install them).

For the tests, also [cargo-nextest](https://nexte.st) 0.9.146: `cargo +stable install --locked cargo-nextest@0.9.146`.

```bash
git clone https://github.com/pirx/voicepuncher
cd voicepuncher
cargo build --release
```

Binary: `target/release/voicepuncher`

## Local services

voicepuncher runs no model itself. It sends each recording to the **speech recognition service for dictation** (OpenAI Audio Transcriptions API) and each raw transcript to a **text refinement service** (OpenAI Chat Completions API), both on this machine, and pastes what comes back. The supported services are whisper.cpp `whisper-server` and llama.cpp `llama-server`. With voice control on, the [command spotting service](#command-spotting-voice-control) confirms utterances with a **speech recognition service for commands** of its own, so the larger dictation model never slows down a voice command; the app never talks to that one.

| Service | Script name | Port | Model (variable) | Copies (variable) |
|---|---|---|---|---|
| speech recognition for dictation | `whisper-server` | 8081 (`VOICEPUNCHER_STT_PORT`) | `ggml-large-v3-turbo-q5_0.bin` (`VOICEPUNCHER_WHISPER_MODEL`) | 3 (`VOICEPUNCHER_STT_INSTANCES`) |
| speech recognition for commands | `whisper-server-commands` | 8085 (`VOICEPUNCHER_COMMAND_STT_PORT`) | `ggml-small.bin` (`VOICEPUNCHER_COMMAND_WHISPER_MODEL`) | 3 (`VOICEPUNCHER_COMMAND_STT_INSTANCES`) |
| text refinement | `llama-server` | 8080 (`VOICEPUNCHER_LLM_PORT`) | `Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf` (`VOICEPUNCHER_LLM_MODEL`) | 1 |
| command spotting | `voicepuncher-command-spotting` | 8082 (`VOICEPUNCHER_KWS_PORT`) | see [Command spotting](#command-spotting-voice-control) | 1 |

```bash
just start-model-servers   # starts all four, in the order of the table (the service for commands before command spotting)
just stop-model-servers    # stops them again (or some: just stop-model-servers llama-server whisper-server-commands)
```

`start-model-servers` checks that `whisper-server`, `llama-server` and `haproxy` are installed (offers `brew install whisper-cpp` / `brew install llama.cpp` / `brew install haproxy`), skips a service if something already answers on its port, and waits until each model is loaded. Each Whisper model is found by its file name in `~/.voicepuncher/` or `~/.voicepuncher/models/`, where `just models` puts it; set the variable (environment or `.env`) to another file to use another model — naming one for dictation leaves the one for commands alone. A default model that is not on disk fails the start with "Run 'just models' or set it in .env". whisper-server also needs the Silero voice activity detection model `ggml-silero-*.bin` (`just models` downloads it; override with `VOICEPUNCHER_VAD_MODEL`): it removes silences longer than 2 s, so a silent recording yields no text, while the pauses between words stay. Logs: `~/.voicepuncher/logs/llama-server-8080.log`, and for a speech recognition service `whisper-server-<port>.log` with one copy (see below for several). Set the port variables of the table to use other ports; PID and log files are per port, so the two speech recognition services never share one. `start-model-servers` also takes the services to start (`just start-model-servers whisper-server`). Each service it launches runs under a supervisor: when the service exits (a crash), it is started again after 1 s (doubling up to 30 s while it keeps crashing), and its log keeps the crash output followed by a `… exited with code N — restarting` note; a service that exits before it was ever ready fails the start at once, with its log. whisper-server runs without language probabilities (`-nlp`): with them, whisper.cpp 1.9.5 crashes on a command confirmation of an utterance without speech. Each speech recognition service runs as three `whisper-server` copies (`VOICEPUNCHER_STT_INSTANCES`, `VOICEPUNCHER_COMMAND_STT_INSTANCES`; 1 runs a single whisper-server) on internal ports 10000 above its own (8081 → 18082–18084, 8085 → 18086–18088), each under its supervisor, with HAProxy on its port in front of them; with five or more copies for dictation, the fifth meets the first copy for commands and the start fails naming that copy's log: whisper-server transcribes one request at a time, and two to four copies on one GPU are about twice as fast on long speech (five and more are slower). A request goes to a free copy or waits; short audio — a command confirmation — waits ahead of long dictation chunks; a copy that crashed gets no requests until it is back. The CPU threads are split between the copies of a service (cores ÷ copies each); logs `~/.voicepuncher/logs/whisper-server-<copy port>.log` and `haproxy-<port>.log` (`haproxy-8081.log`, `haproxy-8085.log`). The service for commands uses the same flags as the one for dictation, with its own model; three copies of small answer the bursts of confirmations of several connections at once (≈ 0.7 GB each; `VOICEPUNCHER_COMMAND_STT_INSTANCES=1` saves ≈ 1.4 GB, unmeasured). `stop-model-servers` stops only servers that `start-model-servers` launched, together with their supervisor, so they stay stopped. A service that is already running keeps the flags it was started with: after an update that changes them, restart it (`just stop-model-servers whisper-server whisper-server-commands && just start-model-servers`).

To run the services by hand instead:

```bash
whisper-server -m ~/.voicepuncher/models/ggml-large-v3-turbo-q5_0.bin \
  --host 127.0.0.1 --port 8081 --inference-path /v1/audio/transcriptions \
  -l auto -bo 1 -nf -sns -mc 0 \
  --vad -vm ~/.voicepuncher/models/ggml-silero-v5.1.2.bin -vsd 2000 -vp 500
# the same with -m ~/.voicepuncher/models/ggml-small.bin --port 8085 for the service for commands
llama-server -m ~/.voicepuncher/models/Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf \
  --host 127.0.0.1 --port 8080 -c 4096 -np 1
```

whisper-server 1.9.5 accepts `-nf` (no temperature fallback) but ignores it: every request still decodes again at temperatures 0.2, 0.4, … 1.0 when the first decoding fails. `-nf` stays on the command line for a whisper-server that honours it. Dictation keeps the server's fallback; a command confirmation turns the retries off per request (`temperature_inc` 0), so on an idle service one confirmation takes at most about 1.5 s.

`-mc 0` gives a request without a prompt no text context: text decoded in one 30 s window of a request does not steer the next. Requests never share text in any case — whisper-server clears it before every request. With no text context whisper.cpp also drops the request's prompt, so a dictation request with a `[transcription] prompt` sets its own context (`max_context` 224) and keeps the prompt first in every window (`carry_initial_prompt`), followed by the text decoded earlier in the same request; whisper.cpp still decodes the last piece of a request shorter than 5 s without either. Command confirmations send no prompt: with the command phrases as the prompt, Russian speech that is no command was heard as a command.

**Writing `[transcription] prompt` for large-v3-turbo:** write it as a sentence that uses the terms, not as a list of them. With the list "Kubernetes, Grafana, Terraform, Kafka." turbo wrote "Kubernetes" in 0 of 36 tries, whatever the server settings; a sentence such as "Мы развернули сервис в Kubernetes, смотрим метрики в Grafana." steers it with every setting tried. The small model follows both forms.

**English terms in Russian dictation:** without a prompt, "oh-my-zsh, zsh и Powerlevel10k" spoken in Russian came out as "а май ZSH, ZSH и по-ирлевелтинке"; with the prompt "В терминале у меня zsh с oh-my-zsh и темой Powerlevel10k, пакеты ставлю через Homebrew и npm." the terms came out as written. There is no default prompt: a Russian prompt also pushes English dictation toward Russian (a short English phrase after a voice command came out as "Кофе машине на 3rd floor", and a long English recording got invented Russian text). Set such a prompt only if you dictate mostly in Russian, and name the terms you use.

The app checks both services at startup and exits with code 2 if one is not reachable, or if the text refinement service offers several models and none matches `[refinement].model`. Only addresses on this machine are accepted (`localhost`, `127.0.0.0/8`, `[::1]`). With `--no-llm` the text refinement service is neither checked nor used. If refinement fails during a recording (service stopped, timeout, cut-off or too short answer), the raw transcript is pasted.

### Command spotting (voice control)

With [voice control](#voice-control) on, the app also needs the **command spotting service** `voicepuncher-command-spotting`, built from this repository (`command-spotting/`): sherpa-onnx keyword spotting with the English `zipformer-gigaspeech-3.3M` model and a Silero voice activity detector. The app streams the microphone to it over a loopback WebSocket while listening; it reports the spotted phrases and where speech starts and ends. Each short utterance (up to 3 s) is also sent to the speech recognition service for commands (`--transcription-url http://127.0.0.1:8085/v1`, the Whisper small model: it answers a short utterance in about 0.25 s, where turbo takes about 1 s and lost commands), and the service reports a command when the whole utterance is a command phrase — this is what recognizes commands in a non-native accent reliably; the keyword spotter alone missed most of them. `just models` downloads both models into `~/.voicepuncher/models/` (override with `VOICEPUNCHER_KWS_MODEL_DIR`, `VOICEPUNCHER_KWS_VAD_MODEL`); `just start-model-servers` builds the service and starts it on `127.0.0.1:8082` (`VOICEPUNCHER_KWS_PORT`), log `~/.voicepuncher/logs/voicepuncher-command-spotting-8082.log`. With voice control on, the app checks it at startup and exits with code 2 when it does not answer.

**When a service stalls or goes away.** The command spotting service answers the app's pings at once, whatever its confirmations do: each confirmation runs on a thread of its own and is given up after 3 s (`--confirmation-deadline-secs`), so a slow speech recognition service for commands never makes voice control stop. When the command spotting service does go away (a crash, a restart, a hang), the app plays the listening-off sound, keeps a running dictation recording and tries the service again every few seconds; once it answers, the listening-on sound plays and the speech of the last seconds is sent to it again, so a "stop dictation" said meanwhile still counts. A speech recognition request that times out or gets an error answer is sent again up to twice; the wait for one request grows with its audio (5 s plus half its length). The scripts also restart a command spotting or text refinement service that stops answering its `/health` check three times in a row (`… stopped answering its health check — restarting` in its log).

**Tracing a session.** Every log line names its thread (`[time LEVEL thread target]`). In the app, `command spotting: session=<n>` follows one connection to the command spotting service (connect, loss with its cause, each reconnect attempt, back with the replayed seconds), `request=<n>` one speech recognition request with each failed attempt, and `voice control: listening → away` / `away → listening again` the state changes. In the command spotting service log, `conn=<n>` follows one app connection over its threads `conn-<n>-io`, `conn-<n>-proc` and `conn-<n>-confirm`, `job=<conn>.<n>` one confirmation (asked, answered in N ms or given up, what it decided), `lag=` audio that waited for processing, and a `stats conn=<n>` line every 30 s. `RUST_LOG=debug` adds every confirmation asked and every skipped check.

### Text refinement

The text refinement service **normalizes** the dictation: punctuation, capitals, misrecognized words, technical terms in their usual spelling (`докер` → `Docker`). Filler sounds (`э-э`, `эм`, `um`, `uh`) are removed by the app before the transcript is sent. Meaning and language stay as dictated; the transcript is data, so a dictated "переведи это на английский" is pasted as that sentence, not translated.

The app pastes the service's answer only when it is still the dictation: the same writing system and at least half of the transcript's Cyrillic and half of its Latin words. Otherwise — a translation, an answer to a dictated question, a rewrite — it logs `refinement answer rejected (…)` and pastes the raw transcript.

The default instruction comes with seven example turns (`[refinement].examples`, each `{ transcript, normalized }`; `examples = []` sends none). A `system_prompt` set in your config replaces the default instruction — delete it to get the current one.

**Speech language:** keep `[transcription].language = "auto"` (the default) when you dictate in more than one language. A fixed language makes the speech recognition service write speech in another language as text in the fixed one: with `language = "ru"`, 29 of 40 English corpus samples came out as Russian, mostly garbled.

## Models

`just models` downloads every model the services need into `~/.voicepuncher/models/` (≈ 2.1 GB the first time) and skips a file already there:

| Model | Used by | File | Source |
|---|---|---|---|
| Whisper large-v3-turbo q5_0 (≈ 574 MB) | speech recognition service for dictation | `ggml-large-v3-turbo-q5_0.bin` | https://huggingface.co/ggerganov/whisper.cpp, pinned revision, sha256 checked |
| Whisper small (≈ 488 MB) | speech recognition service for commands | `ggml-small.bin` | https://huggingface.co/ggerganov/whisper.cpp, pinned revision, sha256 checked |
| Silero VAD | both speech recognition services | `ggml-silero-v5.1.2.bin` | https://huggingface.co/ggml-org/whisper-vad |
| Qwen3-4B-Instruct-2507 Q4_K_M (≈ 2.5 GB) | text refinement service | `Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf` | https://huggingface.co/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF, pinned revision, sha256 checked |
| keyword spotting + Silero VAD | command spotting service | `sherpa-onnx-kws-*`, `silero_vad.onnx` | https://github.com/k2-fsa/sherpa-onnx |

A pinned download whose sha256 does not match is removed, and `just models` fails naming both hashes, so a cut or altered file never lands. The turbo q5_0 file is the measured one: WER 0.077 against small's 0.116 on the fixed 100 corpus recordings, and every chunk-seam case under difficult recording conditions within the bar.

## Quick start

```bash
cp app/config.example.toml ~/.voicepuncher/config.toml
just run
```

`just run` does everything in one command: downloads missing models, restarts the four local services (the command spotting service is rebuilt from the checkout; each service is started again if it crashes while the app runs), builds the app and starts it with voice control; the hotkey records as push-to-talk too. Extra arguments go to the app: `just run --no-llm`. Restarting reloads the models, so it takes a few seconds more than `just start-model-servers` followed by `cargo run --release -p voicepuncher`. It is also how an existing setup picks up a new model: after an update that changed one, `just run` downloads it and restarts the services on it.

Default hotkey is `RightOption`.

## How to use

**Hotkey** (`[hotkey] key`, default `RightOption`):

- `recording_mode = "Toggle"`: press to start a dictation, press again to stop — the text is pasted at the cursor.
- `recording_mode = "Hold"`: hold while speaking, release to paste.
- `Escape` during a dictation cancels it; presses shorter than `min_press_ms` (200 ms) are ignored.

**Voice** (`--voice-control`; `just run` turns it on). The app listens all the time; the hotkey keeps working.

| Say | When | Does |
|---|---|---|
| `start` / `start dictation` | no dictation | starts a dictation |
| `stop` / `stop dictation` | dictating | stops and pastes |
| `cancel` / `cancel dictation` | dictating | drops the dictation |
| `restart` / `restart dictation` | dictating | drops it and starts again |
| `enter` / `press enter` | any time | presses Return; while dictating: stops, pastes, then Return |
| `clear` / `clear text` / `clear all text` | no dictation | Cmd+A, Delete |

A command works when it is:

- said on its own, with a short pause (0.3 s) around it;
- the last words of a phrase: "…and that is all, stop";
- `start` at the beginning: "start, the meeting moved to Friday";
- doubled anywhere, no pause needed: "stop stop", "enter enter";
- a chain of 2–3 commands: "stop enter", "stop clear enter", "stop enter start".

Any command may end with "please". The same words inside a sentence ("we should stop the server") do nothing. Details: [Voice control](#voice-control).

## Voice control

Hands-free dictation, off by default. Turn it on with `--voice-control` or in the config:

```toml
[voice_control]
enabled = true
```

Push-to-talk keeps working as without voice control: the hotkey starts and stops a dictation, Escape cancels it, the overlay and sounds are the same. **Listening** starts with the app (sound `Tink`) and stays on until the app quits; the microphone stays open and six English commands take effect:

| Say | While | Effect |
|---|---|---|
| `start dictation` or `start` | listening, no dictation | starts a dictation (start sound, overlay) |
| `stop dictation` or `stop` | dictating | ends it; the text is pasted without the command |
| `cancel dictation` or `cancel` | dictating | discards it; nothing is pasted |
| `restart dictation` or `restart` | dictating | discards it and starts a new dictation at once |
| `press enter` or `enter` | listening | presses Return in the focused app; during a dictation it first ends it and pastes the text, like "stop enter" |
| `clear all text`, `clear text` or `clear` | listening, no dictation | presses Cmd+A, then Delete, in the focused app |

Every command may end with "please" ("press enter please"). A pause of 0.3 s ends an utterance, also after a quiet word and when more speech follows the pause at once ("restart", a short breath, the next sentence). A command counts:

- **on its own**, with a pause before and after it;
- **at the end of an utterance**, right after other speech: "…and that is all, stop" ends the dictation and pastes the speech before it; the same words inside a sentence ("we should stop the server") are ignored;
- **"start" at the start of an utterance**: "start, the meeting moved to Friday" starts a dictation that begins with "the meeting…";
- **doubled anywhere**, with no pause around it: "start start", "stop stop", "cancel cancel", "restart restart", "enter enter", "clear clear"; after "stop stop" the words said after it are dropped;
- **as a chain** of two or three commands in a row, anywhere: "stop enter" pastes and sends, "stop clear", "cancel clear", "stop enter start" pastes, sends and starts the next dictation; "start" can only come last.

A command repeated within one utterance ("press enter, press enter") counts once.

A dictation started by voice begins where the command's words end, not when the command is recognized a moment later. What you say right after "start", "start start", a chain ending in "start" or "restart" is kept from its first word: the app takes it from the sound it hears all the time while listening.

A command said in the other state is ignored. `press enter` said right after `stop dictation` waits until the text is pasted. The hotkey and the voice commands drive the same dictation: "stop dictation" ends one the hotkey started, and the hotkey ends one "start dictation" started. `press enter` and `clear all text` play the command sound (`Morse`); the sounds are `[sounds]` settings.

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
--voice-control           Voice control: spoken commands drive dictation alongside the hotkey
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

`beam_size`, `patience`, `entropy_thold`, `logprob_thold`, `no_speech_thold` and `flash_attn` are now `whisper-server` launch flags; `temperature_inc` has no flag (whisper-server 1.9.5 ignores `-nf` and keeps its own temperature fallback). New keys: `model` (sent as-is), `temperature`, `timeout_secs` (per request, default 60). Then run `just start-model-servers`.

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
| `just test` | unit + architecture + integration tests under cargo-nextest, each within its time limit | ~1 min | after every edit |
| `just check` | fmt, clippy, `test`, release build, E2E smoke, WER accuracy on 10 samples, refinement evaluation | ~8.5 min (machine lock ≈ 7.3 min) | often while developing, before every commit you push |
| `just full-check` | fmt, clippy, `test`, release build, E2E smoke, chunk seams, outages (`e2e-resilience`), WER accuracy on a fixed sample of 100 recordings (or 10, below), refinement evaluation | ~15.5–17.5 min (machine lock ≈ 14.5 min with 10 recordings, ≈ 16.3 min with 100) | before merging a branch and before every release |

Each step also has its own recipe: `fmt`, `lint`, `build`, `e2e-smoke`, `e2e-quick` (10 samples), `e2e-seams` (the seam sweep, plus Russian and English speech under 11 recording conditions, transcribed in chunks), `e2e-bench` (100 samples, two at a time), `e2e-resilience` (the `@resilience` scenarios: a speech recognition service paused while voice control runs, a command spotting service killed or paused and started again by the scripts, a request abandoned in the queue — each on service ports of its own, so the shared services are never touched), `e2e` (smoke + seams + 100 samples + refinement evaluation), `refine-eval` (the `@refinement` scenarios: dictated commands, normalization, and a per-category score of the 41 dictations in `e2e-test/refinement/eval_set.jsonl` against the live text refinement service, next to the raw-transcript baseline; fails when a dictated command is carried out or the language changes). The E2E recipes start each local service that is not running and stop it afterwards, also when a test fails; a service that is already running — your everyday one included — is reused and left running. `check` and `full-check` start the services once for all their E2E stages and end with each stage's time. Speech synthesized with `say` is cached in `~/.cache/voicepuncher/speech/`; delete it to synthesize everything again.

**Time limits.** Every Rust test runs under [cargo-nextest](https://nexte.st) 0.9.146 with a time limit, set in `.config/nextest.toml`: 2 min a test by default, and each E2E suite its own (one Gherkin harness run counts as one test, e.g. 7 min for the `@smoke` scenarios, 3 min for `e2e-quick`). A test still running at its limit is stopped and reported `TIMEOUT`, and the recipe fails at once instead of hanging. Without cargo-nextest 0.9.146 every test recipe stops with the install command above. `VOICEPUNCHER_HANG_PROBE=1 just test` and `VOICEPUNCHER_HANG_PROBE=1 just e2e-quick` arm two probes that hang on purpose, to see the limits work: they end with `TIMEOUT` after 2 and 3 min. Measured on Apple Silicon (10 cores): `just test` 67 s, `just check` ≈ 8.5 min.

**Models and corpus** are found automatically: each Whisper model by its file name (`ggml-large-v3-turbo-q5_0.bin`, `ggml-small.bin`, `Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf`) in `~/.voicepuncher/` or `~/.voicepuncher/models/`, and the corpus in `e2e-test/corpus/`. Override with `VOICEPUNCHER_WHISPER_MODEL`, `VOICEPUNCHER_COMMAND_WHISPER_MODEL`, `VOICEPUNCHER_LLM_MODEL`, `VOICEPUNCHER_BENCH_CORPUS` (env or `.env`). If they are missing, `check`, `full-check` and the E2E recipes stop with an error instead of skipping. First-time setup: `just models`, `just corpus`.

**The machine-wide test lock.** Checks from several checkouts on one machine take turns through agent-kit's `with-machine-lock`. `check` and `full-check` run fmt, clippy, `test`, the release build and the E2E test builds right away, without the lock (every build at a low CPU priority, the test binaries at the normal one), and wait for the lock only for their stages on the local services (E2E, accuracy, refinement evaluation), which start and stop the services inside it. `just test` and `just release` never wait for it. `full-check` builds incrementally like `check` and runs no `cargo clean`.

**The accuracy sample of `full-check`:** a fixed sample of 100 recordings when the branch changes the recognition path (`recognition_paths` in the `justfile`), on `main` itself and when the changes cannot be read; else 10, like `check`. It says which and why before its first stage. `VOICEPUNCHER_ACCURACY_SAMPLE=100 just full-check` forces 100, e.g. after a new model.

**Your own voice.** Voice control is tested on recordings of your own commands when they exist in `~/.voicepuncher/voice-samples/` (never committed): `<session>.wav` (16 kHz mono, e.g. `rec -r 16000 -c 1 -b 16 session-1.wav` from Homebrew `sox`) with `<session>/manifest.jsonl`, one line per command said (`file`, `start`, `end`, `said`, `command`, `note`, optional `unclear`). `just e2e-smoke` then checks that every clear recording is recognized as its command, with no extra command, and runs whole hands-free sessions with your recorded commands.

### Release checklist

1. `just full-check` passes (on `origin/main` itself it runs the 100 recordings).
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
   - [ ] Voice control (`cargo run --release -p voicepuncher -- --voice-control`): the app plays the listening-on sound at start; in a chat input "start dictation", a sentence, "stop dictation" → the sentence is pasted without the stop phrase; "press enter" → the message is sent; "start dictation", speech, "cancel dictation" → nothing is pasted; type text, "clear all text" → it is erased; "I will press enter later" said without pausing → nothing happens; "start", a sentence, "stop" → the sentence is pasted; "start start" and a sentence at once, then "stop" → the whole sentence is pasted from its first word; "start", a short pause, a sentence, "stop" → the same; the hotkey starts a dictation with the overlay and start sound, and "stop dictation" ends it; "start dictation", then the hotkey → pasted, listening stays on. The log shows each `voice control: "…"` line.
   - [ ] `just stop-model-servers voicepuncher-command-spotting`, then start with `--voice-control` → it exits naming the command spotting service and `just start-model-servers`.
3. After the merge, on a clean checkout of `origin/main` with a bumped `version` in `app/Cargo.toml`: `just release` — cleans `target/`, runs the tests and builds from scratch without the machine-wide lock, then tags `vX.Y.Z` and publishes a GitHub Release with the binary tarball and its SHA-256. In the main checkout, `target/release/voicepuncher` is gone until it finishes.

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
