## 1. Streaming engine core (command-spotting)

- [x] 1.1 Red: unit tests for Cyrillic respelling (commands, glued doubles, fillers, other words untouched) in `command-spotting/src/streaming/respell.rs`
- [x] 1.2 Green: implement `respell` from `research/bench/common.py` `RU_CMD`/`RU_FILL`
- [x] 1.3 Red: unit tests for tokens to timed words in `command-spotting/src/streaming/words.rs`
- [x] 1.4 Green: implement timed words
- [x] 1.5 Red: unit tests for the decision rules (doubled at once, two-word form at once, one-word form after settling, at end only at close, early start, skip before the last report) in `command-spotting/src/streaming/decide.rs`
- [x] 1.6 Green: implement the decision on `PhraseMatcher`
- [x] 1.7 Run `just test`; commit

## 2. Gate, session and trace (command-spotting)

- [x] 2.1 Red: session unit tests with a scripted gate and recognizers (no recognizer runs in silence, replay of 0.5 s at onset, hangover then close, first recognizer wins once, speech events) in `command-spotting/src/streaming/session/tests.rs`
- [x] 2.2 Green: implement `SpeechGate`, `StreamingRecognizer` and the session
- [x] 2.3 Red: unit tests for the trace, its log line and its history entry (`streaming` field, outcome) in `command-spotting/src/streaming/trace.rs` and `command-spotting/src/history/entry.rs`
- [x] 2.4 Green: implement the trace, the log line and the entry
- [x] 2.5 Run `just test`; commit

## 3. Service wiring and real models (command-spotting)

- [x] 3.1 Red: argument tests for `--recognizer`, `--en-model-dir`, `--ru-model-dir` in `command-spotting/src/main.rs`
- [x] 3.2 Green: sherpa-onnx adapter, `Listening` over both sessions in the server, arguments
- [x] 3.3 Add the need `needs_streaming_models` to `.config/stages.toml`, the model discovery to `justfile`, the need to `AGENTS.md`
- [x] 3.4 Red: integration test hearing synthesized "press enter" through the real models in `command-spotting/tests/it/streaming/mod.rs` (`needs_streaming_models`), and the user's recordings (`needs_voice_samples`)
- [x] 3.5 Green: make it pass
- [x] 3.6 Run `just test`; commit

## 4. App configuration, services and models

- [x] 4.1 Red: component scenarios "Voice control starts the streaming recognizer by default" and "The keyword spotter can still be chosen" in `app/tests/features/component/local_services.feature`, steps in `app/tests/steps/local_services/`
- [x] 4.2 Green: `[voice_control] recognizer` in `app/src/config/`, `default.toml`, the service catalog skipping the commands Whisper service, streaming arguments in `catalog/args.rs`; existing keyword spotting scenarios choose the keyword spotter
- [x] 4.3 Red: parity test for the streaming command line in `dev-tools/src/services/tests.rs`
- [x] 4.4 Green: `dev-tools` streaming command line under `VOICEPUNCHER_COMMAND_RECOGNIZER=streaming`
- [x] 4.5 Red: component scenarios "The streaming recognizer needs the streaming models" and "The keyword spotter needs its own models" in `app/tests/features/component/model_download.feature`
- [x] 4.6 Green: the two archives in `app/src/core/domain/model_catalog.rs`, needs by recognizer
- [x] 4.7 Run `just test`; commit

## 5. E2E

- [x] 5.1 Red: the eight voice-control scenarios in `e2e-test/tests/features/voice_control.feature` (`@smoke`), steps in `e2e-test/tests/steps/voice_control/streaming/`, driving the streaming engine in the test process
- [x] 5.2 Green: make them pass
- [x] 5.3 Update `docs/ubiquitous-language.md` (streaming recognizer, voice gate, command recognizer) and `README.md`
- [x] 5.4 Run `just test`; commit

## 6. Release

- [x] 6.1 Bump `version` in `app/Cargo.toml` to 0.42.0
- [ ] 6.2 Run `just check`; commit
- [x] 6.3 Manual checks: none (no hotkey, recording, overlay, paste or permission change); the user may run voice control and paste the command spotting log
