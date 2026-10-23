## Why

Speech recognition (whisper large-v3-turbo) already produces text good enough to paste as is: the user finds the LLM cleanup step unnecessary. The text refinement stage through the local `llama-server` (Qwen3-4B-Instruct-2507) only adds latency between stop and paste, keeps a third model service running on the GPU, and makes every check longer (refinement evaluation, the 20-run output-limit series, the refinement smoke part). Everything related to the LLM is dead weight and should leave the project.

## What Changes

- Dictation pastes the transcribed text directly; no text refinement step runs, ever.
- Remove all production code of text refinement: the refinement port, the llama-server adapter, the prompt, the pipeline wiring, the `--no-llm` flag.
- Remove the LLM settings from the configuration file (and the default configuration written on first launch); an old configuration file that still holds them keeps loading.
- The app and `just start-model-servers` / `just stop-model-servers` no longer start, adopt, supervise or stop `llama-server`; port 8080 and test ports 28080 / 27080 are no longer used.
- Remove LLM model discovery (`VOICEPUNCHER_LLM_MODEL`, the Qwen gguf) and its download from `just models`.
- Remove every LLM test: unit, integration, `text_refinement.feature` (slice/component and E2E), the refinement evaluation stage, the `e2e-smoke-refinement` part, the 20-run series, related benchmarks.
- Remove the related `justfile` recipes and stages, stage inputs, and bundle/release packaging of the llama engine.
- Update docs: `AGENTS.md`, `README.md`, `docs/` (architecture, ubiquitous language), and the OpenSpec specs of text refinement.

Current stage: PoC. Target stage: PoC.
Scope: every LLM-related piece of code, tests, configuration, service management, recipes, packaging and documentation in the repository.
Acceptance bar: `grep -ri 'llama\|llm\|refine'` over the repository finds no live reference (archived OpenSpec changes excepted); `just full-check` passes without any refinement stage; a dictation cycle logs `transcribed` → `pasting…` with no refinement step and the stop→paste time is shorter than before.
Out of scope: the two whisper speech recognition services and the command spotting service; recognition accuracy tuning.

## Capabilities

### Modified Capabilities

- `text-refinement`: removed entirely.
- dictation / pipeline capability: pastes the transcription without refinement.
- service management capability: manages three services instead of four.

## Impact

- `app/` (refinement port, llama adapter, config, CLI flag, service manager), `e2e-test/` (refinement scenarios, evaluation, benchmark), `justfile`, bundle packaging, `.config/` stage inputs and nextest limits, `AGENTS.md`, `README.md`, `docs/`, `openspec/specs/`.

## Open points

- Old configuration files with LLM keys: proposed — ignore unknown keys silently (or with one warning line) instead of failing to load.
- Capability names of dictation and service management in `openspec/specs/`: to be confirmed at the specs stage.
- Whether to remove the downloaded Qwen model file from `~/.voicepuncher/`: proposed — leave the user's files alone, mention it in the release notes.

## Release

SemVer: MINOR — removes a capability (breaking change pre-1.0).
Target version: 0.38.0.
