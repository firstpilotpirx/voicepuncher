# Tasks

## 1. Copies behind one address

- [x] 1.1 Red: in `local_services.feature`, the scenario "Requests that arrive at once are shared by the copies" (`@smoke @serial`) with its steps in `e2e-test/tests/steps/local_services.rs`; the existing scenarios' steps start one copy (`VOICEPUNCHER_STT_INSTANCES=1`). Run it and see it fail: one `whisper-server` answers every request.
- [x] 1.2 Green: `start_whisper` starts the copies under the supervisor, generates and checks the HAProxy configuration and starts HAProxy under the supervisor; `stop_service whisper-server` stops HAProxy and every copy. The scenario passes, and so do the existing `local_services` scenarios.
- [x] 1.3 Red: the scenario "The user stops the speech recognition service that runs as several copies"; run it before 1.2's stop part and see copies left (or confirm it fails for the expected reason).
- [x] 1.4 Green: stopping removes every copy, HAProxy, the copy list and the configuration.
- [x] 1.5 `just test` and commit (the `just check` of the E2E groups runs once, in 3.3, since every check queues on the machine lock).

## 2. A copy that stops

- [x] 2.1 Red: the scenario "One copy of the speech recognition service dies"; run it and see it fail for the expected reason (no copy found, or the copy does not come back).
- [x] 2.2 Green: HAProxy health checks keep requests away from the restarting copy; the supervisor restarts it. The scenario passes.
- [x] 2.3 `just test` and commit.

## 3. Short speech first

- [x] 3.1 Red: the scenario "A short utterance overtakes long speech that waits" with two copies; run it without the priority rule and see the short request finish after the queued long one.
- [x] 3.2 Green: `set-priority-class` on `Content-Length` under 160 000 bytes. The scenario passes.
- [x] 3.3 `just check` and commit.
- [x] 3.4 Review fixes: the overtake scenario sends four long requests and is red without the priority rule; the dying-copy scenario sends four requests right after the kill (red: 1 of 4 answered) and HAProxy marks a copy down at its first refused connection (`observe layer4`, `option redispatch 1`); the stop scenario checks that the copy list and configuration are removed; a single-copy start removes a stale copy list; new scenario "The second of several copies cannot start" (second copy's port held, red with the cleanup trap off: the first copy was left) and "The user stops a single speech recognition service after copies were left listed" (red without the stale-list removal: the service still answered). The ten `local_services` scenarios pass.

## 4. Docs, timings, version, manual checks

- [x] 4.1 README (§ local services, `just run`, requirements) and AGENTS.md: the copies, `VOICEPUNCHER_STT_INSTANCES`, HAProxy.
- [x] 4.2 Time `just check` and `just full-check` on a fresh pool of three copies (spare ports) against `speed-up-checks`' research; `just e2e-bench` three runs in a row; add the numbers to `research/README.md`.
- [x] 4.3 Bump `version` in `app/Cargo.toml` to 0.12.0; `just check` and commit.
- [x] 4.4 Manual checks by the user: none (no change to hotkey, recording, overlay, paste or permissions; voice commands go through the same address).
