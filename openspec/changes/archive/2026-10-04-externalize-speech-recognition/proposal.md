# Proposal

## Why

voicepuncher runs Whisper inside its own process: it links whisper.cpp through `whisper-rs`, loads a 1.6 GB model at startup and exposes a dozen decoder knobs in its config. Running a model is not the app's job — mature local servers (whisper.cpp `whisper-server`, Speaches, LocalAI) already do it behind a standard API. Owning the engine makes every build compile C++ with Metal, slows startup, and ties the app to one engine and one model format.

## What Changes

- The app sends each recording (or live chunk) to a **local speech recognition service** over the OpenAI Audio Transcriptions API and reads back the transcript. The user starts that service separately, normally with `just start-services` (below); the app never launches it.
- Supported service: whisper.cpp `whisper-server` (Homebrew `whisper-cpp`). It is the reference open-source Whisper engine, runs on Metal on every Apple Silicon chip, and loads the `ggml-*.bin` models users already have. Other servers that implement the same API may work but are neither tested nor documented.
- Two `just` recipes manage the local services. This change introduces them for `whisper-server`; `externalize-text-refinement` adds `llama-server` to the same recipes.
  - **`just start-services`**: checks that `whisper-server` is installed and, in an interactive terminal, offers to install it with Homebrew (without a terminal it fails with the install command instead); starts the service with the Whisper model the user already has unless one already answers on its port; waits until it is ready.
  - **`just stop-services`**: stops only the services `start-services` launched and leaves services started by hand untouched.
- **BREAKING** — config: the `[whisper]` section is replaced by `[transcription]` with the service address, model name, language, prompt, temperature and timeout. Decoder keys (`model_path`, `threads`, `beam_size`, `patience`, `temperature_inc`, `entropy_thold`, `logprob_thold`, `no_speech_thold`, `flash_attn`) move to the service's own launch flags. A config that still uses them fails with a message naming the replacement.
- **BREAKING** — CLI: `--whisper-model <path>` is replaced by `--transcription-url <url>`.
- The service address must be on this machine (loopback); any other host is rejected, so audio never leaves the machine.
- At startup the app checks that the service is reachable and exits with an actionable message if it is not.
- Whisper hallucination filtering and live chunking stay in the app and work unchanged with any service.
- The in-process engine is removed: the `whispercpp` adapter and the `whisper-rs` dependency go away; the setup wizard stops downloading the Whisper model and asks for the service address instead.
- The E2E recipes (`just e2e`, and the E2E parts of `just check`) run `start-services` before the tests and `stop-services` after them, also when the tests fail.

## Capabilities

### New Capabilities

- `speech-recognition`: turning recorded speech into a raw transcript through a local speech recognition service — service location and privacy, availability, language, hallucination filtering, long recordings.

### Modified Capabilities

None (no specs exist yet).

## Impact

- **Stage:** current PoC → target PoC. This is an architecture change inside the PoC; no new user-facing feature.
- **Scope:** the `start-services` and `stop-services` recipes, the transcription adapter, its config section and CLI flag, startup checks, the setup wizard's Whisper step, the build dependencies, `justfile`, E2E tests and docs.
- **Acceptance bar:**
  - `just check` passes with `whisper-rs` removed from `app/Cargo.toml`.
  - `just e2e` mean raw WER on the corpus is ≤ 6.5 % (5.5 % today).
  - The app starts without any Whisper model on disk when the service is running.
  - With the service stopped, the app exits within 3 s, naming the service address.
  - The manual checklist in `README.md` passes.
- **Out of scope:** the app itself starting, stopping or supervising the service; documenting or testing servers other than `whisper-server`; remote or cloud endpoints; streaming transcription; changing the chunking strategy; text refinement (see `externalize-text-refinement`).
- **Code:** `app/src/adapters/secondary/transcription/` (new `openai/`, remove `whispercpp/`), `app/src/config.rs`, `app/src/main.rs`, `app/src/adapters/primary/cli/`, `app/src/adapters/primary/setup/cli/`, `app/tests/`, `e2e-test/tests/`, `justfile`, `README.md`, `CLAUDE.md`, `docs/ubiquitous-language.md`.
- **Dependencies:** removes `whisper-rs` (and the whisper.cpp C++/Metal build); reuses `ureq` and `hound`, which the app already has, adds `serde_json` (already used by `e2e-test`) for the response and `url` (already in the tree through `ureq`) for the loopback check.
- **Runtime:** the app now opens a loopback HTTP connection. CLAUDE.md's "zero network I/O" constraint becomes "no internet; loopback only".
