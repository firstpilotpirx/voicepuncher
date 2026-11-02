## 1. Move the voice tests to the streaming recognizer

- [x] 1.1 Red: `should_report_a_synthesized_paste` in `command-spotting/tests/it/streaming` and the "эйст" row and "ест" guard in `streaming/respell/tests.rs` fail
- [x] 1.2 Green: respell "эйст"/"ейст" as "paste" (design D2)
- [x] 1.3 `dev-tools` starts the command spotting service only with the streaming recognizer; `VOICEPUNCHER_COMMAND_RECOGNIZER` goes (design D3); its unit test follows
- [x] 1.4 The E2E harness stops choosing the keyword spotter in `voice_control/mod.rs`, `local_services/managed.rs` and `app_distribution/bundle_launch/mod.rs`
- [x] 1.5 Red: run the smoke, cues, resilience and bundle E2E stages on the streaming recognizer and list every failure of a moved scenario
- [x] 1.6 Green: fix each failure test-first (design D2): "start", a short pause and a sentence; a sentence, a short breath and "stop stop"; the hands-free session's pasted "stop"; the user's quiet recorded "restart" after a pause; the user's recorded "press enter"; the start cues over their bounds (the English model's lag and the close of an utterance waiting for it, which holds the next one back); and any further failure of 1.5
- [x] 1.7 `voice_samples`, `voice_forms` and the adapter tests of `commandspotting_websocket` run against the streaming service; the tests of keyword spotter events among them go to group 2
- [x] 1.8 Commit: `test: move the voice tests to the streaming recognizer`

## 2. Delete the tests of the keyword spotter and the duplicates

- [x] 2.1 Delete the E2E scenarios, steps and helpers of audit group 1 (`voice_control.feature`, `local_services.feature`, `speech_recognition.feature`, `spotting_service/`, `stalled_copies`, `tests/it/confirmation_time.rs`), and the `.feature` rules of the requirements the delta specs remove or replace
- [x] 2.2 Delete the `command-spotting/tests/it` modules of group 1 (`engine/`, `placed_commands/`, `early_start/`, `stall.rs`, `mishearings/`, the confirmation test of `server.rs`, `kws_models.rs`, `scripted_hearing/`, `stream_clock/`) and move the protocol tests of `server.rs` to the streaming service
- [x] 2.3 Delete the app and dev-tools tests and component scenarios of group 1 (confirmation switch, keyword spotter choice and models, failed confirmation, stalled service, pause rule over spotted phrases) and rewrite the catalog tests' `keyword_spotting()` helper to the streaming recognizer
- [x] 2.4 Delete the duplicates of audit group 3 (four `command-spotting/tests/it/streaming` tests, the paste scenario that repeats the short-form row, the four `commandspotting_websocket` tests the E2E outlines repeat)
- [x] 2.5 `.config/nextest.toml` loses the `command-spotting-engine` group and its filters; `.config/stages.toml` loses `needs_command_stt` and `needs_kws_models`
- [x] 2.6 Rename the Gherkin rules of the replaced requirements to their new names, and move the two timing scenarios under a new rule "The start cue is timed on an idle machine", so `spec_mirror` passes
- [x] 2.7 Commit: `test: delete the keyword spotter tests`

## 3. Delete the keyword spotter from the command spotting service

- [x] 3.1 Delete `engine/`, `confirm/`, `worker/` (moving `Wake` if streaming uses it), `outbox/`, `orderer/`, `keywords/` (keeping `UnknownPhrases`) and `keywords.txt`; move `TimedWord` to `streaming`
- [x] 3.2 `main.rs` keeps only the streaming arguments (design D3); `listening` goes and the server serves the streaming engine directly
- [x] 3.3 The protocol drops `Phrase`, `Heard`, `ConfirmationFailed`, `Stalled`, `Timing`, `Verdict` and `confirm_commands`, and moves to version 4 (design D4); `command-spotting` drops its copy-pool dependency
- [x] 3.4 Commit: `refactor: delete the keyword spotter`

## 4. Delete the keyword spotter and the service for commands from the app

- [ ] 4.1 Red: a configuration naming `recognizer`, `confirm_commands`, `command_copies` and `commands_url` still loads
- [ ] 4.2 Green: those keys are accepted and ignored; `CommandRecognizer` and the fields go (design D5)
- [ ] 4.3 Delete `ServiceKind::Commands`, `catalog/keyword_spotting.rs`, the Whisper branch of `catalog::needed`, the Whisper small and keyword spotting entries of the model catalog, the settings switch and its hint, `default.toml` keys (design D6)
- [ ] 4.4 The websocket client, the supervisor and `VoiceCommandRecognizer` act only on the streaming events; `SpottingEvent::{PhraseSpotted, ConfirmationFailed}`, `ConfirmationTiming` and the `confirm=` stage go
- [ ] 4.5 Commit: `refactor: delete the service for commands from the app`

## 5. Recipes, docs and version

- [ ] 5.1 `dev-tools` loses the service for commands; the `justfile` loses `command_model_file`, `command_stt_url`, the KWS and command Whisper variables and the service for commands in `start-` and `stop-model-servers`
- [ ] 5.2 `README.md`, `AGENTS.md`, `docs/ubiquitous-language.md` and `docs/architecture/` no longer name the keyword spotter, the confirmation or the service for commands
- [ ] 5.3 Bump `version` in `app/Cargo.toml` to 0.46.0
