## Why

Voice commands registered only on the 2nd or 3rd try with the keyword spotter: quiet speech, a Russian accent and doubled words ("stop stop") came out hyphenated or glued, and each command waited for a Whisper confirmation. Version 0.42.0 (`streaming-command-recognition`) added the streaming command recognizer — a voice gate waking an English and a Russian streaming recognizer — and made it the default (`[voice_control] recognizer = "streaming"`), keeping the keyword spotter as a fallback. Once the streaming recognizer has proved itself in daily use, the fallback and the speech recognition service it alone needs are dead weight: a second recognizer path to maintain and test, a third local service, and the ≈ 488 MB Whisper small model.

## What Changes

- The keyword spotter is gone: `[voice_control] recognizer` and its settings control go away, the streaming recognizer is the only one.
- The speech recognition service for commands (`whisper-server-commands`, `ggml-small.bin`) is no longer started, downloaded or listed.
- Fewer services and models: voice control needs only the command spotting service with its streaming models.

Current stage: PoC. Target stage: PoC.
Scope: the `keyword_spotting` value, the keyword spotting engine and its Whisper confirmation in `command-spotting/`, their catalog entries, models, service scripts, test needs, E2E scenarios and docs.
Acceptance bar: voice control works with only the dictation service and the command spotting service running; no code, recipe, model or doc refers to the keyword spotter or the speech recognition service for commands; `just full-check` passes.
Out of scope: the copy pool, the speech recognition service for dictation, the streaming recognizer's own behavior.

## Capabilities

### Modified Capabilities

- `voice-control`: commands are heard only by the streaming recognizer; the keyword spotting and confirmation requirements are removed.
- `local-services`: the speech recognition service for commands and the recognizer choice are removed.
- `model-download`: the Whisper small and keyword spotting models are no longer needed.

## Impact

- `command-spotting/` (keyword spotting engine, confirmation, `--recognizer`), `app/` (config, settings window, service and model catalogs), `dev-tools/` (service start), `justfile`, `.config/stages.toml`, `e2e-test/`, `README.md`, `docs/ubiquitous-language.md`.

## Open points

- When the streaming recognizer counts as proved: proposed, after the user has used 0.42.x daily without falling back to `keyword_spotting`; the user decides.
- Whether `[voice_control] recognizer` stays readable (ignored) for old configuration files: proposed, an unknown key is ignored, so it may simply go.
- Whether the user's recorded voice samples suite is rerun on the streaming recognizer before the removal: proposed, yes.

## Release

SemVer: MINOR — removes a capability option (breaking for a configuration that chose `keyword_spotting`, pre-1.0).
Target version: 0.43.0.
