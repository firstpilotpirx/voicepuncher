## 1. Start only the named services

- [x] 1.1 Red: add `local_services.feature` (`@local_services @e2e`, rule `@smoke @serial`) with the scenario "Only the speech recognition service is started" and its steps in `e2e-test/tests/steps/local_services.rs` (free ports, a guard that stops what the scenario started); run it and see it fail because `start-model-servers` starts all three services
- [x] 1.2 Green: `start-model-servers *services` starts only the named services, all three when none is named, and rejects an unknown name
- [x] 1.3 `just check` and commit

## 2. Audio without speech never stops the speech recognition service

- [x] 2.1 Red: add the scenario "Command confirmation of an utterance without speech" to `speech_recognition.feature` (`@smoke`), with its steps (the command spotting service's own confirmation request, via a dev-dependency on `voicepuncher-command-spotting`); run it and see the service crash; commit the intentionally red test on its own
- [x] 2.2 Green: start `whisper-server` with `-nlp`, with a comment on why
- [x] 2.3 `just check` and commit

## 3. Keep the services running

- [x] 3.1 Red: add the scenario "The speech recognition service dies while voice control runs" and its steps; run it and see the killed service stay down
- [x] 3.2 Green: run each service under a supervisor loop in `start-model-servers` (restart with back-off, restart note appended to the log, log kept across restarts)
- [x] 3.3 Red: add the scenario "The user stops the speech recognition service" and its steps; run it and see the supervisor start the stopped service again
- [x] 3.4 Green: the supervisor stops its service and exits on `TERM`/`INT`; `stop-model-servers` recognizes a supervisor by its command line and kills its children in the `kill -9` fallback
- [x] 3.5 Red: add the scenario "The speech recognition model is not a model" and its steps; run it and see the start hang until its timeout or leave a supervisor behind
- [x] 3.6 Green: `start_service` reports a service that exits before it was ever ready as a start failure, stops its supervisor and removes the PID file
- [x] 3.7 `just check` and commit

## 4. Docs, version and manual checks

- [x] 4.1 README (§ local services, `just run`) and AGENTS.md: services are restarted when they stop, `start-model-servers` takes service names, `-nlp` and why
- [x] 4.2 Bump `version` in `app/Cargo.toml` to 0.11.0
- [x] 4.3 `just check` and commit
- [x] 4.4 Manual checks by the user: none (no change to hotkey, recording, overlay, paste or permissions)
