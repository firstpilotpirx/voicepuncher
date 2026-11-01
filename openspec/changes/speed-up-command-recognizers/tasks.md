## 1. Faster recognizers

- [x] 1.1 Red: add "Press enter after a sentence is reported in time with every core busy" to `e2e-test/tests/features/voice_control.feature` with its busy-cores step in `e2e-test/tests/steps/voice_control/streaming/`; run it and see it fail
- [x] 1.2 Red: unit test in `command-spotting/src/main.rs` for 1 streaming thread by default
- [x] 1.3 Green: default 1 in `main.rs`; the decoding thread at the user-interactive class in `command-spotting/src/streaming/session/decoder.rs`
- [x] 1.4 Green: `sherpa-onnx` with the `shared` feature in `command-spotting/Cargo.toml`; build scripts adding the run paths in `command-spotting/build.rs` and `e2e-test/build.rs`
- [x] 1.4a Remove the "please" form: keyword lines, `TRAILING_WORDS` in `voice-commands/src/lib.rs`, polite words in `app/src/core/domain/command_words/`, the «сто плиз» respelling, and every "please" test, scenario and Example row; rename the start cue `Rule:` after its requirement
- [x] 1.4b The keyword spotter's start trigger threshold to 0.10 in `command-spotting/keywords.txt`; the start cue scenarios pass
- [x] 1.5 Bundle both libraries beside the command spotting program in `dev-tools/src/engines/staging.rs`
- [x] 1.6 Streaming bench: the time it waited for the service per second of audio in `command-spotting/src/bin/streaming_bench.rs`
- [x] 1.7 README: threads and the shared library of the command spotting service
- [x] 1.8 `just test`, commit

## 2. Release

- [x] 2.1 Bump `version` in `app/Cargo.toml` to 0.44.0
- [ ] 2.2 `just check`, commit
- [x] 2.3 Manual checks: none
