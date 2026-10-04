# Proposal

## Why

voicepuncher runs its text-cleanup LLM inside its own process: it links llama.cpp through `llama-cpp-2`, loads the GGUF model at startup, hard-codes the Qwen chat template and runs the token loop itself. Serving an LLM is not the app's job — `llama-server`, LM Studio, Ollama and MLX all serve local models behind the de-facto standard OpenAI Chat Completions API. Owning the engine costs a C++/Metal build (including the OpenSSL workaround), slow startup, and lock-in to one model family's prompt format.

## What Changes

- The app sends each raw transcript to a **local text refinement service** over the OpenAI Chat Completions API, with the configured system prompt, and pastes the returned text. The user starts that service separately, normally with the start script below; the app never launches it.
- Supported service: llama.cpp `llama-server` (Homebrew `llama.cpp`). It is the reference open-source LLM engine (Ollama and LM Studio run on it), runs on Metal on every Apple Silicon chip, and loads the `.gguf` models users already have. Other servers that implement the same API may work but are neither tested nor documented.
- `just start-services` and `just stop-services` (added by `externalize-speech-recognition`) manage `llama-server` next to `whisper-server`:
  - **start**: checks that `whisper-server` and `llama-server` are installed and offers to install the missing ones with Homebrew; starts each service that is not running yet and skips one that already answers; waits until both are ready.
  - **stop**: stops the services the start script launched and leaves services started by hand untouched.
- **BREAKING** — config: the `[llm]` section is replaced by `[refinement]` with the service address, model name, optional API key, system prompt, temperature, token limit and timeout. `model_path` and `context_size` move to the service's own launch flags. A config that still uses them fails with a message naming the replacement.
- **BREAKING** — CLI: `--llm-model <path>` is replaced by `--refinement-url <url>`. `--no-llm` keeps working.
- The service address must be on this machine (loopback); any other host is rejected, so transcripts never leave the machine.
- At startup the app checks that the service is reachable and that the configured model is available, and exits with an actionable message if not (skipped with `--no-llm`).
- The current fallbacks stay: if refinement fails, times out, or returns truncated text, the raw transcript is pasted.
- The in-process engine is removed: the `llamacpp` adapter and the `llama-cpp-2` dependency go away; the setup wizard stops downloading the LLM and asks for the service address and model name instead. With no model left to download, the model downloader goes too, so the app holds no code that reaches the internet.
- The E2E recipes start the text refinement service with the speech recognition service and stop it afterwards; the model comparison (`just bench`) starts `llama-server` per LLM model.

## Capabilities

### New Capabilities

- `text-refinement`: correcting a raw transcript into cleaned text through a local text refinement service — service location and privacy, availability, prompt contract, fallbacks, opting out.

### Modified Capabilities

None (no specs exist yet).

## Impact

- **Stage:** current PoC → target PoC. This is an architecture change inside the PoC; no new user-facing feature.
- **Version:** MINOR (breaking config and CLI change before 1.0): `0.2.0` → `0.3.0`.
- **Scope:** the start and stop scripts, the refinement adapter, its config section and CLI flag, startup checks, the setup wizard's LLM step, the build dependencies, `justfile`, E2E tests and docs.
- **Acceptance bar:**
  - `just check` passes with `llama-cpp-2` removed from `app/Cargo.toml`.
  - `just e2e` mean WER with LLM on the corpus is ≤ 7.2 % (6.2 % today).
  - The app starts without any LLM file on disk when the service is running.
  - With the service stopped, the app exits within 3 s, naming the service address.
  - With the service stopped mid-session, a recording still pastes the raw transcript.
  - The manual checklist in `README.md` passes.
- **Out of scope:** the app itself starting, stopping or supervising the service; documenting or testing servers other than `llama-server` (LM Studio, Ollama, MLX); remote or cloud endpoints (OpenAI, Anthropic); the Anthropic Messages API; streaming responses; prompt changes; speech recognition (see `externalize-speech-recognition`).
- **Code:** `app/src/adapters/secondary/textrefinement/` (new `openai/`, remove `llamacpp/`), `app/src/config.rs`, `app/src/config/`, `app/src/main.rs`, `app/src/adapters/primary/cli/`, `app/src/adapters/primary/setup/cli/` (model downloader removed), `app/tests/`, `e2e-test/tests/`, `e2e-test/scripts/bench.py`, `justfile`, `README.md`, `AGENTS.md`, `docs/ubiquitous-language.md`, `docs/project-map.md`.
- **Dependencies:** removes `llama-cpp-2` (the last C++ engine: the llama.cpp C++/Metal build and the Homebrew OpenSSL paths in `justfile` go with it) and `indicatif` (used only by the model downloader); reuses `ureq`, `url` and `serde_json`, which `externalize-speech-recognition` added.
- **Runtime:** a second loopback HTTP connection; AGENTS.md already says "no internet; loopback connections only".
- **Builds on:** `externalize-speech-recognition` (landed): its loopback validator, `start-services`/`stop-services` recipes and fatal config errors are reused.
