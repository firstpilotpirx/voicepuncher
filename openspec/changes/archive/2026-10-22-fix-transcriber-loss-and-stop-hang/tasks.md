# Tasks

## 1. A dead worker loses only its dictation (design decisions 1, 2)

- [x] 1.1 Red: component scenario "The worker dies mid-dictation" in `app/tests/features/component/speech_recognition.feature` (new `Rule:`) with its steps in `app/tests/steps/speech_recognition.rs`; run it, see it fail, commit it alone (`test: reproduce …`, intentionally red).
- [x] 1.2 Green: `SharedTranscriber` in `app/src/core/application/pipeline.rs`; drop `TranscriberSource`, `dropped_worker`, `reclaim_transcriber`; a disconnected worker notes `DictationLost(SpeechRecognition)`; no `panic!`/`expect` on the transcriber path.
- [x] 1.3 `just test`; commit.

## 2. Bounded stop wait (design decision 3)

- [x] 2.1 Red: component scenario "The speech recognition request never answers" in `speech_recognition.feature` with its steps; run it, see it fail, commit it alone (intentionally red).
- [x] 2.2 Green: `Pipeline::with_stop_wait`, `recv_timeout` in `stop_recording` and `discard_recording`; `OpenAiTranscriber::longest_wait` and its unit test; wiring sets the bound.
- [x] 2.3 `just test`; commit.

## 3. A failed pipeline stops in order and starts again (design decision 4)

- [x] 3.1 Red: component scenarios "The pipeline panics once" and "The restarted pipeline panics again at once" in `app/tests/features/component/app_runtime.feature` (new `Rule:`) with steps in `app/tests/steps/app_runtime.rs`; run them, see them fail, commit them alone (intentionally red).
- [x] 3.2 Green: `catch_unwind` in `Next::launch`, `Pipeline::stop_after_failure`, `Command::PipelineFailed` with the generation, the supervisor's restart and `RESTART_WINDOW`; no `process::exit` in the pipeline thread.
- [x] 3.3 `just test`; commit.

## 4. Version

- [x] 4.1 Bump `version` in `app/Cargo.toml` to 0.36.1. Manual checks: a dictation with the speech recognition service paused on stop (README § Release checklist), deferred to the batch's manual run.
