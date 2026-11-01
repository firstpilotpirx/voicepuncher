## 1. Faster recognizers

- [ ] 1.1 Red: add "Press enter after a sentence is reported in time with every core busy" to `e2e-test/tests/features/voice_control.feature` with its busy-cores step in `e2e-test/tests/steps/voice_control/streaming/`; run it and see it fail
- [ ] 1.2 Red: unit test in `command-spotting/src/main.rs` for 1 streaming thread by default
- [ ] 1.3 Green: default 1 in `main.rs`; the decoding thread at the user-interactive class in `command-spotting/src/streaming/session/decoder.rs`
- [ ] 1.4 Green: `sherpa-onnx` with the `shared` feature in `command-spotting/Cargo.toml`; build scripts adding the run paths in `command-spotting/build.rs` and `e2e-test/build.rs`
- [ ] 1.5 Bundle both libraries beside the command spotting program in `dev-tools/src/engines/staging.rs`
- [ ] 1.6 Streaming bench: the time it waited for the service per second of audio in `command-spotting/src/bin/streaming_bench.rs`
- [ ] 1.7 README: threads and the shared library of the command spotting service
- [ ] 1.8 `just test`, commit

## 2. Release

- [ ] 2.1 Bump `version` in `app/Cargo.toml` to 0.43.1
- [ ] 2.2 `just check`, commit
- [ ] 2.3 Manual checks: none
