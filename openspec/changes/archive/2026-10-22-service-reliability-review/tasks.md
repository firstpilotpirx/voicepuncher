## 1. Health-check every speech recognition copy

- [x] 1.1 Red: catalog unit test that every whisper copy spec is health-checked (`catalog/tests`)
- [x] 1.2 Green: set `health_checked` for whisper copies in `catalog/mod.rs`; update the domain doc in `core/domain/local_service.rs`
- [x] 1.3 Red: supervision unit test that a hung copy is killed and restarted while the others keep running (`supervision_tests.rs`)
- [x] 1.4 Green: make it pass; `just test`; commit

## 2. Wait for a copy to come back

- [x] 2.1 Red: `copy-pool` unit test for `CopyPool::await_copy` (returns once a copy is probed up; false after the wait)
- [x] 2.2 Green: add `await_copy` in `copy-pool/src/pool.rs`
- [x] 2.3 Red: `transcription_openai_it.rs` test: every copy down, one comes back → transcript; none comes back → error after the wait, both logged
- [x] 2.4 Green: recovery wait in `transcription/openai/mod.rs` (`RECOVERY_WAIT`, test constructor for a short wait); `just test`; commit

## 3. Text refinement as copies with retry

- [x] 3.0 Red/Green: `copy-pool` `ServiceAddress::with_copy_offset` and `CopyPool::shared_at`, with a unit test that no two default services share a copy port
- [x] 3.1 Red: catalog unit tests: refinement on → two llama copies on copy ports; one configured copy on the own port; adopted when the own port answers
- [x] 3.2 Green: `[services] refinement_copies` (config, `default.toml`, clamp 1–4) and llama copies in `catalog/mod.rs`
- [x] 3.3 Red: `textrefinement_openai_it.rs` tests: killed copy → other copy refines; every copy down then one back → refined, wait logged; silent copy → other copy after the time limit
- [x] 3.4 Green: `CopyPool` leasing, retry on refused/dropped/timeout and the recovery wait in `textrefinement/openai/mod.rs`; existing drop tests stay green; `just test`; commit

## 4. Command spotting as copies with failover

- [x] 4.1 Red: catalog unit test: voice control on → two command spotting copies on copy ports
- [x] 4.2 Green: `[services] command_spotting_copies` and command spotting copies in `catalog/mod.rs`
- [x] 4.3 Red: `commandspotting_reconnect_it.rs` test: two service stubs on copy ports, the connected one closes → the client is back on the other at its first attempt, with no long-outage sign
- [x] 4.4 Green: `SpottingClient::connect` tries the own port then the copy ports (`websocket/client.rs`)
- [x] 4.5 Red: `command-spotting/src/confirm.rs` test: a refused copy → the confirmation is answered by the other copy, refusal logged
- [x] 4.6 Green: failover loop in `UtteranceTranscriber::transcribe`; `just test`; commit

## 5. Resilience E2E with the app's own supervisor

- [x] 5.1 Add `ManagedServices` start with a given `Machine` (`runtime/services.rs`) and an E2E helper that runs it on scenario ports (`e2e-test/tests/steps/local_services/managed.rs`)
- [x] 5.2 Red: `local_services.feature` @resilience scenario "An owned speech recognition copy that stops answering is restarted" with steps
- [x] 5.3 Red: `speech_recognition.feature` @resilience scenario "Every copy of the dictation service is killed while a chunk is sent" with steps
- [x] 5.4 Red: `text_refinement.feature` @resilience scenario "A copy of the text refinement service is killed during a refinement" with steps
- [x] 5.5 Red: `voice_control.feature` @resilience scenarios "The command spotting copy the app listens with is killed" and "A copy for commands is killed while voice control runs" with steps
- [x] 5.6 Green: `cargo build -p voicepuncher-e2e-test --tests`; `just test`; commit (the scenarios run in the batch `full-check`)

## 6. Docs and release

- [x] 6.1 Update `docs/ubiquitous-language.md` (Copy, Copy pool cover every service), `default.toml` comments and README services section
- [x] 6.2 Bump `version` in `app/Cargo.toml` to 0.36.0; `just test`; commit
- [x] 6.3 Manual checks by the user: none (no hotkey, recording, overlay, paste or permissions change)
