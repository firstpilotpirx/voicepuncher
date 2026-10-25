# Tasks

## 1. Reproduce the bug

- [x] 1.1 Red: adapter integration test in `app/tests/it/transcription_openai/` — a foreign listener on the service's own port and two stand-in copies; the transcription goes to the copies and the foreign listener gets no request; commit it alone as `test: reproduce …` (intentionally red)

## 2. The copy pool identifies its copies

- [x] 2.1 Red: copy-pool unit tests — a scripted probe with a foreign service port, a foreign copy port, an unanswered identity question that is asked again, a port that stops and comes back asked again
- [x] 2.2 Green: `Probe::serves` (default `true`), the HTTP/1.1 `GET /` identity probe, identity remembered per port in the pool state, `CopyPool::shared` with the HTTP probe
- [x] 2.3 Red: unit test of reading `lsof -Fpc` output into a program name and pid; Green: `copy-pool` looks up the holder and logs one warning per foreign port
- [x] 2.4 `PoolError::NoCopies` carries the foreign program on the service's own port; copy-pool version 0.4.0
- [x] 2.5 Shared stand-in helper in `test-steps` that answers `GET /` naming whisper; every speech recognition stand-in in `app/tests`, `command-spotting` and `e2e-test` uses it
- [x] 2.6 The reproduction test 1.1 is green; Red/Green: foreign copy port passed over, and the log warning once per port, in `app/tests/it/transcription_openai/`
- [x] 2.7 `just test`; commit

## 3. The notice names the program on the port

- [x] 3.1 Red: `failure_notices.feature` scenarios "Another program holds the dictation port and no copy runs" and "Nothing listens on the dictation port and no copy runs", with their steps
- [x] 3.2 Red: adapter integration test — a foreign listener on the service's own port and no copy; the error is `ServicePortTaken` naming the port, the program and this process's pid
- [x] 3.3 Green: domain `ForeignProgram`, `ServicePortTaken`, `FailingPart::ServicePortTaken`; `FailingPart` and `FailureNotice` drop `Copy`
- [x] 3.4 Green: the transcription adapter maps the pool error; processing, recording and the live worker carry the cause to the notice
- [x] 3.5 `just test`; commit

## 4. End to end and documents

- [x] 4.1 Red: `e2e-test/tests/features/local_services.feature` scenario "Another program on the service's own port is passed over" with its steps (a foreign listener on the test service port beside the three real copies)
- [x] 4.2 Green: run it in the `e2e-smoke-services` stage of `just check`
- [x] 4.3 `docs/ubiquitous-language.md` (copy, copy pool, foreign program), `AGENTS.md` and `README.md` copy-port wording and the identity check
- [x] 4.4 `just test`; commit

## 5. Release

- [x] 5.1 Bump `version` in `app/Cargo.toml` to 0.38.7; `just check`; commit
- [x] 5.2 Manual checks: none (no hotkey, recording, overlay layout, paste or permission change; the notice text is covered by the component scenarios)
