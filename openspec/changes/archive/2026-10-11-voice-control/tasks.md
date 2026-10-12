# Tasks

## 1. Voice commands and the pause rule (domain)

- [x] 1.1 Red: unit tests in `app/src/core/domain/voice_command.rs` — each of the five phrases maps to its `VoiceCommand` and back; an unknown phrase maps to none; the standby and dictating phrase sets hold exactly the phrases the spec allows; run `just test` and see them fail
- [x] 1.2 Green: add `VoiceCommand`, `SpokenCommand` (a command with its phrase start `Instant`) and the per-state phrase sets; `just test` passes
- [x] 1.3 Red: unit tests in `app/src/core/domain/voice_command_recognizer.rs` — a phrase that fills its speech segment (≤ 0.3 s margins) yields its command at `speech_end`; extra speech before or after the phrase in the segment yields nothing (command phrase inside a sentence); two phrases in one segment yield nothing; a phrase with no segment end yet yields nothing; see them fail
- [x] 1.4 Green: implement `VoiceCommandRecognizer`; `just test` passes
- [x] 1.5 Add *voice control*, *listening*, *voice command*, *command spotting service* and *key press* to `docs/ubiquitous-language.md`
- [x] 1.6 `just check` and commit

## 2. Ports, config and sounds

- [x] 2.1 Red: config loader tests in `app/src/config.rs` — `[voice_control]` absent → disabled with the default loopback URL; `enabled = true` and a custom loopback URL are kept; a non-loopback URL is rejected; the three new `[sounds]` keys default to `Tink`, `Bottle`, `Morse`; `--voice-control` turns it on (`app/tests/cli_it.rs`); see them fail
- [x] 2.2 Green: add `VoiceControlCfg`, the sound keys and the CLI flag; document them in `app/config.example.toml`; `just test` passes
- [x] 2.3 Add ports `CommandSpotter` (`start_listening`, `listen_for`, `stop_listening`, `probe`) and `KeyPresser` (`press_enter`, `clear_all_text`) in `app/src/core/application/`, `PipelineInput` (`Hotkey`, `Spotting`, `SpottingLost`) and `SpottingEvent`; `SoundPlayer` gains `play_listening_on`, `play_listening_off`, `play_command` (macOS adapter plays the configured sounds); `AudioRecorder` gains `truncate_at(cut)` (default: no cut, as the filesystem recorder needs); `just test` passes
- [x] 2.4 Red: unit tests for `CpalAudioRecorder` sample-index mapping — a cut `Instant` 1 s after capture start keeps 16 000 samples; a cut before the start keeps none; a cut after the end keeps all; see them fail
- [x] 2.5 Green: implement `truncate_at` in `CpalAudioRecorder` (stop appending, drop samples after the cut); `just test` passes
- [x] 2.6 `just check` and commit

## 3. Pipeline voice control and component feature

- [x] 3.1 Switch `Pipeline::run` to `Receiver<PipelineInput>`; `main` forwards hotkey `PttSignal`s into it; the focus wait queues non-hotkey inputs for `run`; existing pipeline unit tests and `.feature` steps send `PipelineInput::Hotkey`; `just test` passes with no behavior change
- [x] 3.2 Red: write `app/tests/features/component/voice_control.feature` (`@voice_control @component`, one `Rule:` per requirement, every scenario the design's test-level table assigns to component) and its steps in `app/tests/steps/voice_control.rs`, driving the real `Pipeline` with a scripted `CommandSpotter` and recording stand-ins for recorder, transcriber, paster, key presser and sound player; register it in `app/tests/cucumber_integration.rs`; run `just test` and see it fail
- [x] 3.3 Red: pipeline unit tests — the hotkey with voice control on turns listening on and off and ignores `Released` in hold mode; `StopDictation` truncates the recording at the phrase start before stopping it; every state change sends its phrase set; `SpottingLost` during a dictation pastes and turns listening off; see them fail
- [x] 3.4 Green: add voice control (a listening flag beside `Idle`/`Recording`) and `with_voice_control(spotter, key_presser)` to `Pipeline`; run the recognizer on `Spotting` inputs, apply the per-state rules, reuse the start / stop / cancel paths (truncate at the phrase start before a stop), call the key presser, play the sounds; `just test` passes
- [x] 3.5 `just check` and commit

## 4. Command spotting service

- [x] 4.1 Add workspace member `command-spotting/` (binary `voicepuncher-command-spotting`) with `sherpa-onnx = "=1.13.8"` and `tungstenite` pinned exactly; `cargo build -p voicepuncher-command-spotting` succeeds
- [x] 4.2 Generate `command-spotting/keywords.txt` once with sherpa-onnx `text2token` from the model's `bpe.model` (tokens, boost and threshold per phrase, extra token lines where a phrase runs together) and note the command used in a comment line of the file
- [x] 4.3 Red: unit tests in the service crate — the phrases message parses; an unknown phrase is rejected with its name; the selected keywords string holds only the requested phrases; service times are seconds since the first sample; see them fail
- [x] 4.4 Green: implement the server (loopback WebSocket, `KeywordSpotter` with per-stream keywords, Silero VAD with 0.4 s minimum silence, events of design decision 1; CLI arguments for port, model directory, VAD model, threads, minimum silence; thresholds in `keywords.txt`); `cargo test -p voicepuncher-command-spotting` passes and `just test` runs it
- [x] 4.5 justfile: `models` fetches the KWS model tarball and `silero_vad.onnx`; `start-model-servers` builds and starts the service on `VOICEPUNCHER_KWS_PORT` (8082) with `VOICEPUNCHER_KWS_MODEL_DIR` / `VOICEPUNCHER_KWS_VAD_MODEL` auto-discovery; `stop-model-servers` and the E2E service helpers know it; `just start-model-servers` then `nc -z 127.0.0.1 8082` succeeds
- [x] 4.6 `just check` and commit

## 5. macOS adapters and wiring

- [x] 5.1 Red: adapter integration test `app/tests/commandspotting_websocket_it.rs` against the real service (URL from env, run by `e2e-smoke`) — `probe` succeeds; a `say`-synthesized "press enter" WAV streamed through the adapter's client yields `speech_start`, a `press enter` phrase and `speech_end`; an unknown phrase set is refused; add it to `_e2e-smoke`; see it fail
- [x] 5.2 Green: implement the adapter in `app/src/adapters/secondary/commandspotting/websocket/` (own cpal capture, 16 kHz mono frames, event parsing, service time → `Instant`, one `SpottingLost` on failure); the test passes
- [x] 5.3 Add `MacOsKeyPresser` in `app/src/adapters/secondary/keypressing/macos/` (Return; Cmd+A, 30 ms, Delete) with `app/tests/keypressing_macos_it.rs` gated like `textpasting_macos_it.rs`
- [x] 5.4 Wire voice control in `app/src/main.rs`: when enabled, `probe` at start and exit 2 with the message of design decision 4 on failure; pass spotter and key presser to the pipeline; architecture test still passes (extend its forbidden-import lists with `tungstenite` and `sherpa_onnx` for the core)
- [x] 5.5 `just check` and commit

## 6. E2E feature

- [x] 6.1 Red: write `e2e-test/tests/features/voice_control.feature` (`@voice_control @e2e @smoke`) — each phrase spoken on its own (`say`) is recognized as its command by the real service and the recognizer; a phrase inside a sentence is not; the app refuses to start with voice control on and no service on the port; steps in `e2e-test/tests/steps/voice_control.rs`, registered in `cucumber_e2e.rs`; run `just e2e-smoke` and see it fail where behavior is missing
- [x] 6.2 Green: fix what the scenarios find (thresholds in `keywords.txt`, startup message); `just e2e-smoke` passes
- [x] 6.3 Document voice control in `README.md` (turning it on, the five commands, `just models` / `just start-model-servers`, the `clear all text` warning, the manual-run steps in § Release checklist) and the third service in `AGENTS.md` (§ What this project is, § How to verify a change, § Key constraints)
- [x] 6.4 `just check` and commit

## 7. Release and manual checks

- [x] 7.1 Bump `version` in `app/Cargo.toml` to 0.7.0; `just check` and commit
- [ ] 7.2 Manual run by the user (`just models`, `just start-model-servers`, `cargo run --release -p voicepuncher -- --voice-control`): the listening-on sound plays at start, no hotkey press; in a chat input say "start dictation", a sentence, "stop dictation" (text pasted without the stop phrase), "press enter" (message sent); "start dictation", speech, "cancel dictation" (nothing pasted); type text and say "clear all text" (erased); say "I will press enter later" without pausing (nothing happens); the hotkey turns listening off (listening-off sound) and on again; the log shows each command and a healthy dictation cycle

## Workflow follow-up

- Verify with `/opsx:verify` and the `spec-test-reviewer` agent, archive, `just full-check`, PR, manual run, merge, release v0.7.0.
