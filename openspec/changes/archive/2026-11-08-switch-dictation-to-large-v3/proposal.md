## Why

The user's dictations are Russian, often fast or unclear, with English terms; the large-v3-turbo model (4 decoder layers, 5-bit quantized) mishears such words. A one-off comparison on 2026-11-08 of seven models on 100 FLEURS Russian recordings and 45 of the user's own dictations (15 with English terms) found the full large-v3 model (16-bit, 32 decoder layers) with beam search 5 the best on the user's speech: about 6 dictations of 45 with a meaning error against about 10 for turbo ("after reload" for turbo's "авторе лоу", "в L2 для Worker… это L3, а не L2" for turbo's "литре они 2"), the same word error rate on FLEURS (5.8 %), at twice the recognition time (1.6 s per 10 s of speech against 0.8 s). GigaAM v3 and Parakeet v3 garbled the English terms.

## What Changes

- Dictation is recognized with Whisper large-v3, 16-bit (`ggml-large-v3.bin`, 3.1 GB), instead of large-v3-turbo q5_0; the app downloads it into the models folder like the other models, from a pinned revision with its sha256.
- The speech recognition service for dictation decodes with beam search of 5 candidates instead of greedy decoding, in the app's service command line and the service scripts'.
- Every test, step and document naming the turbo model names the new one; the README's notes on the model, its memory (3 copies ≈ 10 GB) and the prompt advice follow.

Current stage: PoC. Target stage: PoC.
Scope: the model catalog, the service command lines of the app and of `dev-tools`, the model discovery of the `justfile`, the tests and documents naming the model.
Acceptance bar: `just check` and the landing group's `full-check` pass with the new model, the accuracy stages within the ratchet; a dictation in the installed app logs `transcribed` with the new model.
Out of scope: the prompt and its default (none, as today), the server's other flags (`-mc 0` stays: the app already sets the context for a request with a prompt), the chunking, voice control, the number of copies, other model families.
Interface: none.

## Capabilities

### Modified Capabilities

- `speech-recognition`: dictation is recognized with the large-v3 model, 16-bit, decoded with beam search of 5, instead of large-v3-turbo.

## Impact

- `app/src/core/domain/model_catalog.rs`, `app/src/adapters/secondary/localservices/catalog/`, `dev-tools/src/services/`, `justfile`, tests and steps naming the model in `app/tests/`, `e2e-test/tests/`, `README.md`, `AGENTS.md`, `docs/`.
- Disk: 3.1 GB more in `~/.voicepuncher/models/`; memory: about 10 GB for three copies.

## Open points

- Beam size: 5, measured; larger was not tried.
- Target version: 0.47.0.

## Release

SemVer: MINOR — dictation runs on another model, a behavior change the user notices in speed and text.
Target version: 0.47.0.
