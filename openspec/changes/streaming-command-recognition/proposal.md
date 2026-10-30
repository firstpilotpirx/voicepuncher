## Why

Voice commands often register only on the 2nd or 3rd try when said quietly or with the user's accent: the keyword spotter catches 29% of the user's commands (58% false alarms) and Whisper confirmation adds ~0.45 s. On 415 of the user's own recordings, an English streaming model (Nemotron 0.6B) plus a Russian one (zipformer small ru) reading the same audio hear 94% of commands at ~0.2 s median delay (`research/README.md`).

## What Changes

- The command spotting service gets a streaming recognizer: a voice gate (Silero) wakes two streaming recognizers, English and Russian, at speech onset, replaying 0.5 s of audio before it; the first to hear a command reports it, at once, with no Whisper confirmation.
- Russian spellings of the English command words ("стоп", "энтер") count as those commands.
- Every gated utterance is logged and kept in the command history: its audio, each recognizer's texts over time, and the time of each stage (onset, first text, command, end of speech, decode load).
- The app chooses the recognizer in its configuration, streaming by default; streaming needs no speech recognition service for commands, so the app no longer starts it, and voice control downloads the two streaming models instead of the Whisper small and keyword spotting models.
- The service scripts can start either recognizer, keyword spotting by default.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `voice-control`: streaming recognition, Russian spellings, the per-stage log.
- `local-services`: the recognizer choice decides whether the speech recognition service for commands is started.
- `model-download`: the streaming models.

## Scope

Stage PoC → PoC. Acceptance bar: the existing checks pass; a command said alone is reported within half a second of audio after it.

Out of scope: removing the keyword spotting path, the speech recognition service for commands and their models (a later change, once streaming proves itself in daily use); retraining or fine-tuning models.

## Impact

`command-spotting/` (new streaming engine, history entries), `app/` (config, service catalog, model catalog), `dev-tools/` (service start), `justfile` (model discovery), `.config/stages.toml` (a test need). SemVer: MINOR, target version 0.42.0.
