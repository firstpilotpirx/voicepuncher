## 1. Corpus and stub services (design 1, 2)

- [x] 1.1 Red: run `transcription_openai_it` with `VOICEPUNCHER_BENCH_CORPUS` pointing at a folder without `manifest.jsonl`; see it fail on the missing file
- [x] 1.2 Green: gate the corpus tests in `app/tests/transcription_openai_it.rs` on the manifest existing
- [x] 1.3 Default `VOICEPUNCHER_BENCH_CORPUS` in `justfile` to the main checkout's corpus when the worktree has none
- [x] 1.4 Set the read timeout after `tungstenite::accept` in the stub servers of `app/tests/commandspotting_client_it.rs` and `app/tests/commandspotting_reconnect_it.rs`
- [x] 1.5 `just test`; commit

## 2. Tests of the real command spotting service (design 3)

- [x] 2.1 Wait for the first command up to 10 s, then 2 s for extras, in `recognized_from_microphone` of `app/tests/commandspotting_websocket_it.rs`
- [x] 2.2 Put `commandspotting_websocket_it`, `voice_forms_it` and `voice_samples_it` in the `command-spotting-models` group in `.config/nextest.toml`
- [x] 2.3 Name each failed request and its error or empty transcript in the "Requests that arrive at once" step of `e2e-test/tests/steps/local_services/copies.rs`, so its next failure shows the cause (the code shows none)
- [x] 2.4 `just test`; commit

## 3. Checks keep their audio when they wait (design 5)

- [x] 3.1 Red: add to `command-spotting/tests/placed_commands_it.rs` a test with hearing delayed 2 s per confirmation: "restart restart" in the middle of dictated speech is reported once; see it fail; commit it alone as intentionally red
- [x] 3.2 Red: add a test there that "stop enter start" gives the same commands with immediate hearing and with hearing delayed 2 s
- [x] 3.3 Green: queue a check that falls due while another waits, with the audio range it was due for, in `command-spotting/src/engine.rs` and `command-spotting/src/engine/spotted.rs`; a later window replaces a queued one
- [x] 3.4 Feed E2E audio on an absolute schedule in `e2e-test/tests/steps/voice_control/session.rs` and `hands_free.rs`
- [x] 3.5 `just test`; commit

## 4. Seam repeats count only spoken pairs (design 6)

- [x] 4.1 Red: unit test in `e2e-test/src/speech.rs`: a pair heard twice that was never spoken is no seam repeat; see it fail
- [x] 4.2 Green: count only pairs that occur in the speech, more often than spoken
- [x] 4.3 `just test`; commit

## 5. No sampling fallback for dictation (design 7)

- [x] 5.1 Red: unit test in `app/src/adapters/secondary/transcription/openai/request.rs`: a dictation request carries `temperature_inc=0`; see it fail
- [x] 5.2 Green: send `temperature_inc=0` with dictation requests
- [x] 5.3 Add to `app/tests/transcription_openai_it.rs` a test that transcribes a noisy corpus recording twice and expects the same text
- [x] 5.4 `just test`; commit

## 6. Version and stability

- [x] 6.1 Bump `version` in `app/Cargo.toml` to 0.22.2
- [x] 6.2 `just check`; commit
- [x] 6.2a Read the dictation sign beside a pointer that stood still between the reads in `app/tests/visualfeedback_macos_it.rs` (a moving real pointer raced the sign's follow timer)
- [x] 6.2b Say the retried "cancel dictation" of the start-cue steps in another voice (`e2e-test/tests/steps/voice_control/session/start_cue.rs`): with deterministic decoding the same audio was misheard the same way twice
- [x] 6.2c Wait in `app/tests/commandspotting_websocket_it.rs` for the audio handed to the stream as well as for the answer (under load 30 s of listening took 10 s to reach the service, the whole command timeout)
- [x] 6.3 Run `just test` 10 times in a row on one commit with no rerun; list the runs in the PR
- [x] 6.3a Run `just full-check` 5 times in a row on that commit with no rerun; list each run in the PR (the user accepted 4 passing runs: 3 on feeba7b, 1 on d296532, which differ only in one test; every other failed run ran beside other sessions loading the machine to 35–70; the start-cue borderline is filed as speed-up-short-start-cue)
- [x] 6.4 Manual checks: none
