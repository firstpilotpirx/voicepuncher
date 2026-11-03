## Why

The app hears commands only with the streaming recognizer (`[voice_control] recognizer = "streaming"`, the default since 0.42.0), but the command spotting binary still defaults to the keyword spotter, the service scripts start it with its Whisper confirmation, and the E2E harness picks it on purpose. So almost every voice control test — about 110 E2E runs, the user's recorded voice, the start cue timings, the outages and the bundle — checks a path the product never takes, while the streaming path the user runs has about 15 scenarios. Each locked run also starts three copies of `whisper-server-commands` that only the keyword spotter needs. The audit of 2026-11-02 counted about 260 tests of the keyword spotter and its confirmation, about 10 duplicates, and about 6 500 lines of code only they use.

## What Changes

- **Tests move to the streaming recognizer first.** The service scripts start the command spotting service with the streaming recognizer, the E2E harness no longer picks the keyword spotter, and every test of behavior the product has (forms, placements, chains, hands-free sessions, the user's recordings, start cues, outages, the bundle's engines, the command history, the protocol) runs on it.
- **The keyword spotter is gone**: its engine, its Whisper confirmation and the copies it confirms on, its worker, outbox, orderer and keyword list; `--recognizer`, `--model-dir`, `--transcription-url`, `--confirmation-deadline-secs` and the related flags; the protocol events only it sends (`Phrase`, `Heard`, `ConfirmationFailed`, `Stalled`) and the `confirm_commands` selection.
- **BREAKING** `[voice_control] recognizer` and `confirm_commands`, `[services] command_copies` and `commands_url` go; the settings window loses the Whisper confirmation switch. An old configuration file that still names them loads, the keys ignored.
- **The speech recognition service for commands is gone**: `whisper-server-commands`, its copies, its port 8085 and `ggml-small.bin` are no longer started, downloaded, listed or required; nor is the keyword spotting model.
- **Every test of the keyword spotter, its confirmation or the service for commands is deleted**, and the duplicate tests the audit found (the same behavior twice at one level).

Current stage: PoC. Target stage: PoC.
Scope: the keyword spotter and its confirmation in `command-spotting/`, the app's recognizer choice, confirmation switch and service for commands, the service scripts, recipes, test needs and nextest groups that serve them, every test of them, the duplicates, docs; moving the voice tests to the streaming recognizer.
Acceptance bar: no code, recipe, model, setting or doc refers to the keyword spotter, the Whisper confirmation of commands or the speech recognition service for commands; every voice control E2E scenario runs on the streaming recognizer; `just full-check` passes.
Out of scope: the copy pool crate (the dictation copies keep it), the speech recognition service for dictation, the streaming recognizer's own behavior, retuning the streaming recognizer beyond what a moved test needs to pass.

## Capabilities

### Modified Capabilities

- `voice-control`: commands are heard only by the streaming recognizer; the keyword spotting, confirmation, confirmation copies and KWS command history requirements are removed; the pause, slowness and sync requirements no longer speak of confirmations.
- `local-services`: the speech recognition service for commands, its confirmations and the recognizer choice are removed; scenarios that used it as their example use another service.
- `speech-recognition`: no command confirmation and no small model.
- `settings-window`: the Whisper confirmation switch is removed.
- `model-download`: the Whisper small and keyword spotting models are no longer needed.
- `menu-bar-presence`: the services listed no longer include the one for commands.

## Impact

- `command-spotting/` (most of `src/` and `tests/it/`), `app/` (config, settings window, service and model catalogs, voice command recognizer, websocket client, start timing), `dev-tools/` (service start), `e2e-test/`, `justfile`, `.config/stages.toml`, `.config/nextest.toml`, `README.md`, `AGENTS.md`, `docs/`.

## Open points

- Old configuration keys: settled — unknown keys are ignored, so a file naming `recognizer` or `confirm_commands` still loads.
- When the streaming recognizer counts as proved: settled by the user on 2026-11-02 ("the product recognizes commands differently, throw the rest out").
- The user's recorded voice samples rerun on the streaming recognizer: yes, they move with the other tests.

## Release

SemVer: MINOR — removes a capability option (breaking for a configuration that chose `keyword_spotting`, pre-1.0).
Target version: 0.46.0.
