## 1. Corpus and stub services (design 1, 2)

- [ ] 1.1 Red: run `transcription_openai_it` with `VOICEPUNCHER_BENCH_CORPUS` pointing at a folder without `manifest.jsonl`; see it fail on the missing file
- [ ] 1.2 Green: gate the corpus tests in `app/tests/transcription_openai_it.rs` on the manifest existing
- [ ] 1.3 Default `VOICEPUNCHER_BENCH_CORPUS` in `justfile` to the main checkout's corpus when the worktree has none
- [ ] 1.4 Set the read timeout after `tungstenite::accept` in the stub servers of `app/tests/commandspotting_client_it.rs` and `app/tests/commandspotting_reconnect_it.rs`
- [ ] 1.5 `just test`; commit

## 2. Tests of the real command spotting service (design 3)

- [ ] 2.1 Wait for the first command up to 10 s, then 2 s for extras, in `recognized_from_microphone` of `app/tests/commandspotting_websocket_it.rs`
- [ ] 2.2 Put `commandspotting_websocket_it`, `voice_forms_it` and `voice_samples_it` in the `command-spotting-models` group in `.config/nextest.toml`
- [ ] 2.3 `just test`; commit

## 3. A busy client is not a silent service (design 4)

- [ ] 3.1 Red: add to `app/tests/commandspotting_client_it.rs` a test that sends 30 s of held audio at once to a stub service answering pings and expects no loss; see it fail; commit it alone as intentionally red
- [ ] 3.2 Green: read answers before the silence check and send held audio in pieces of at most 1 s in `app/src/adapters/secondary/commandspotting/websocket/`
- [ ] 3.3 `just test`; commit

## 4. Checks keep their audio when they wait (design 5)

- [ ] 4.1 Red: add to `command-spotting/tests/placed_commands_it.rs` a test with hearing delayed 2 s per confirmation: "restart dictation" during a dictation is reported once; see it fail; commit it alone as intentionally red
- [ ] 4.2 Red: add a test there that "stop enter start" gives the same commands with prompt hearing and with hearing delayed 2 s
- [ ] 4.3 Green: queue a check that falls due while another waits, with the audio range it was due for, in `command-spotting/src/engine.rs` and `command-spotting/src/engine/spotted.rs`; a later window replaces a queued one
- [ ] 4.4 Feed E2E audio on an absolute schedule in `e2e-test/tests/steps/voice_control/session.rs` and `hands_free.rs`
- [ ] 4.5 `just test`; commit

## 5. Seam repeats count only spoken pairs (design 6)

- [ ] 5.1 Red: unit test in `e2e-test/src/speech.rs`: a pair heard twice that was never spoken is no seam repeat; see it fail
- [ ] 5.2 Green: count only pairs that occur in the speech, more often than spoken
- [ ] 5.3 `just test`; commit

## 6. No sampling fallback for dictation (design 7)

- [ ] 6.1 Red: unit test in `app/src/adapters/secondary/transcription/openai/request.rs`: a dictation request carries `temperature_inc=0`; see it fail
- [ ] 6.2 Green: send `temperature_inc=0` with dictation requests
- [ ] 6.3 Add to `app/tests/transcription_openai_it.rs` a test that transcribes a noisy corpus recording twice and expects the same text
- [ ] 6.4 `just test`; commit

## 7. Version and stability

- [ ] 7.1 Bump `version` in `app/Cargo.toml` to 0.18.1
- [ ] 7.2 `just check`; commit
- [ ] 7.3 Run `just test` 10 times and `just full-check` 5 times in a row on one commit with no rerun; list each run in the PR
- [ ] 7.4 Manual checks: none
