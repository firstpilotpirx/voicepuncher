# Changelog

All notable changes to this project are documented here. Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html); before 1.0.0 a breaking change bumps the minor version.

## [0.3.1] — 2026-10-04

### Fixed

- Dictations longer than the chunk threshold (30 s) no longer paste a phrase twice at a chunk seam when the speech recognition service transcribes the overlapping two seconds differently in the two chunks (punctuation, case, spelling, a word cut at the chunk edge, a misread word).

### Changed

- Recording files (`--input-file`) longer than the chunk threshold are transcribed in chunks, like a dictation.

## [0.2.0] — 2026-10-04

### Changed — BREAKING

- Speech recognition runs in a separate local service (whisper.cpp `whisper-server`) instead of inside the app. The app sends each recording over the OpenAI Audio Transcriptions API to an address on this machine; any other host is rejected.
- Config: the `[whisper]` section is replaced by `[transcription]` (`base_url`, `model`, `language`, `prompt`, `temperature`, `timeout_secs`, chunking keys). A config that still has `[whisper]` makes the app exit with code 2 and names the replacement of every key. See `README.md` § Upgrading from the in-process Whisper engine.
- CLI: `--whisper-model <PATH>` is replaced by `--transcription-url <URL>`.
- An invalid config now stops the app with exit code 2 instead of silently falling back to defaults.

### Added

- `just start-services` / `just stop-services` start and stop `whisper-server` (and offer to install it with Homebrew); the E2E recipes run them around the tests.
- The app exits with code 2 within 3 s, naming the address, when the speech recognition service is not reachable at startup.
- The setup wizard asks for the speech recognition service address instead of downloading a Whisper model.

### Removed

- The in-process whisper.cpp engine and the `whisper-rs` dependency — the build no longer compiles whisper.cpp.

## [0.1.0]

- Initial PoC: push-to-talk recording, in-process whisper.cpp transcription, Qwen2.5 refinement via llama.cpp, paste at the cursor.
