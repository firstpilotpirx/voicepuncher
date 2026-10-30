## 1. Replay bench

- [x] 1.1 Red: unit tests for the bench's clip labels and latency summary (median, p90, accuracy, false alarms) in `command-spotting/src/bench/`
- [x] 1.2 Green: `streaming-bench` binary streaming voice samples, labelled history entries and negatives through the streaming session in real time; `just streaming-bench` recipe
- [x] 1.3 Record the baseline (current code, Nemotron 2 threads) in `research/README.md` of this change
- [x] 1.4 `just test`; commit

## 2. Independent recognizers

- [x] 2.1 Red: unit test "a slow recognizer does not hold back the other" in `streaming/session/tests.rs` (scripted slow English model)
- [x] 2.2 Green: one worker thread per recognizer in `streaming/session/`, reports de-duplicated as now
- [x] 2.3 Bench run; record the result
- [x] 2.4 `just test`; commit

## 3. English recognizer speed and choice

- [x] 3.1 Bench Nemotron at 2, 4, 6 threads and with decode threads at user-interactive QoS; record
- [x] 3.2 Bench the Nemotron 80 ms chunk variant and Parakeet Realtime EOU 120M (when sherpa-onnx 2.0.1 loads it); record
- [x] 3.3 Red: test that the chosen setup's threads and QoS reach the model (`streaming/sherpa/`) and, for a new model, that the model catalog lists it (`app/` model catalog test)
- [x] 3.4 Green: apply the chosen setup; new model archive in the model catalog and service command line when one wins
- [x] 3.5 `just test`; commit

## 4. Lone command at the pause, shorter tail

- [x] 4.1 Red: unit tests in `streaming/decide/tests.rs`: "stop" alone settles at the gate's pause; "stop the car" still no command
- [x] 4.2 Red: E2E scenarios "start said alone", "stop said alone", "press enter after a sentence" within their wall-clock bounds in `e2e-test/tests/features/voice_control.feature` (`@cues`)
- [x] 4.3 Green: settle on the gate's pause in `streaming/decide/` and `streaming/session/`
- [x] 4.4 Sweep the utterance tail on the bench; Green: set the shortest tail that keeps every command
- [x] 4.5 `just test`; commit

## 5. Quiet speech and missed utterances

- [x] 5.1 Red: integration test "a quiet command opens the gate" in `command-spotting/tests/it/streaming/`
- [ ] 5.2 Sweep gate gain and threshold on the bench; Green: set the chosen pair in `streaming/sherpa/`
- [x] 5.3 Red: scenario "An utterance without a command is kept too" in `e2e-test/tests/features/voice_control.feature` with its steps
- [x] 5.4 Green: keep every gate-opened utterance in `streaming/session/kept.rs`, outcome `none`
- [ ] 5.5 `just test`; commit

## 6. Mishearings and accuracy

- [x] 6.1 List recurring mishearings of the user's commands from the bench's misses
- [x] 6.2 Red: unit tests for those mishearings (the Russian ones are respellings: `command-spotting/src/streaming/respell/`)
- [x] 6.3 Green: add them to the respellings, a glued double split with backtracking; the English ones stay out (research README § Mishearings)
- [ ] 6.4 Red/Green: integration tests "the user's recorded commands are reported within their bound" and "are heard" (`needs_voice_samples`) in `command-spotting/tests/it/streaming/`
- [x] 6.5 Update `docs/ubiquitous-language.md` (streaming bench, outcome `none`)
- [ ] 6.6 `just test`; commit

## 7. Version and checks

- [ ] 7.1 Bump `version` in `app/Cargo.toml` to 0.42.1
- [ ] 7.2 `just test`; `just check`; commit
- [ ] 7.3 Manual checks: none (no hotkey, recording, overlay, paste or permission change)
