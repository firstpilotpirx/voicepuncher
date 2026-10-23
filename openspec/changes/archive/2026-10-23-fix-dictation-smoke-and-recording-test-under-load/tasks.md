# Tasks

## 1. The recording keeps its audio while the drain is held up

- [x] 1.1 Red: in `app/tests/it/recording_cpal/synthetic.rs`, the busy-machine test asserts first that no sample was dropped, naming the dropped audio in ms and the wall time of the minute, and the held-up test holds the drain for 5 s on a ring of the adapter's `RING_SECONDS`; run them, see the held-up test fail on the 2 s ring
- [x] 1.2 Green: `RING_SECONDS` 2 → 10 in `app/src/adapters/secondary/recording/cpal/capture.rs`; the busy-machine test builds its ring from the constant; both tests pass
- [x] 1.3 `.config/nextest.toml`: an override giving the busy-machine test a 3 min limit (2 × 84 s); `just test` green
- [x] 1.4 `just test`, commit

## 2. The smoke scenarios transcribe one file at a time and name the load

- [x] 2.1 Red: a unit test in a new `e2e-test/src` module that reads the copies that sent no answer from the app's log; run it, see it fail
- [x] 2.2 Green: the module passes; the file-mode When steps in `e2e-test/tests/steps/file_transcription.rs` and `dictation_archive.rs` store `machine_load::report()` with those copies in the world when the log names one; the Then steps that print the app's log (`speech_recognition.rs`, `speech_recognition/languages.rs`, `speech_recognition/prompt.rs`, `file_transcription.rs`, `dictation_archive.rs`) print the report
- [x] 2.3 `@serial` on every `@smoke` scenario that transcribes a recording file in `e2e-test/tests/features/speech_recognition.feature`, `file_transcription.feature` and `dictation_archive.feature`; `just e2e-smoke` green
- [x] 2.4 `.config/nextest.toml`: the `e2e-smoke-speech` limit set to twice the longest run the stage journal shows after 2.3
- [x] 2.5 `just test`, commit

## 3. Acceptance series

- [x] 3.1 Measure: `just e2e-smoke` once beside `just e2e-regression` from the main checkout: the speech stage passed (harness 56.7 s), the run failed on the local-services copies scenario, filed as `fix-copies-shared-scenario-under-load`, which takes the 3-run series (the user's decision of 2026-10-23)
- [x] 3.2 Measure: the busy-machine recording test 20 times in a row inside `just test` (`VOICEPUNCHER_REUSE_STAGES=0`); the stage journal shows no failed run
- [x] 3.3 Record both series (runs, longest times) in `design.md` § Risks; `just test`, commit

## 4. Version and checks

- [x] 4.1 `version` 0.37.3 → 0.37.4 in `app/Cargo.toml`; `just test`, commit
- [x] 4.2 Run `just check` once
- [x] 4.3 Manual checks: a dictation of a minute on a loaded machine with `cargo run --release -p voicepuncher`: the log shows no "ms of audio dropped" and the pasted text is whole
