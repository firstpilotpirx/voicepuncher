# Tasks

## 1. Wait for the command spotting service, not for the clock

- [x] 1.1 Red: in `app/tests/commandspotting_client_it.rs` (new: `commandspotting_websocket_it.rs` is already over the file budget), add a stub service that answers each binary frame with a `speech_start` event after 200 ms, and `should_return_every_event_for_the_audio_sent_once_the_service_answered_it`: three frames sent, the wait returns three events. Add the `SpottingClient` method as `todo!()`; `just test` fails on it.
- [x] 1.2 Red: in the same file, `should_fail_the_wait_when_the_service_stops_answering` against a hanging stub service; it fails on the `todo!()`.
- [x] 1.3 Green: implement the method in `app/src/adapters/secondary/commandspotting/websocket/client.rs` (ping with its own payload, read until its pong, return the events read, fail after the silence limit); both tests pass, `just test` is green.
- [x] 1.4 Red (bug), not reproducible: `spotting_events` streaming 3 min of audio (more than the 8 MB the loopback socket buffers hold) to a stub service that stops reading for 3 s, then 5 s, after the first frame passes on the current code — the flake needs a sustained backlog plus a long stall of the real service under load. No test is committed for it; three `just e2e-bench` runs in a row (6.2) are its check.
- [x] 1.5 Green: `spotting_events` and `carried_out_by_app` in `e2e-test/src/voice_commands.rs` wait for the service every 2 s of audio and at the end (`carried_out_by_app` after every frame) instead of the 4 s quiet time; `just test` is green.
- [x] 1.6 `app/tests/voice_samples_it.rs` waits for the service at the end of each utterance and every 2 s of audio instead of its 1.5 s quiet time; `app/tests/commandspotting_websocket_it.rs` tests that stream through `spotting_events` keep passing. `just e2e-smoke` (inside the `just check` of 2.4) passes; `voice_forms_it` and `voice_samples_it` report their times.
- [x] 1.7 Commit `test(voice-control): wait for the command spotting service instead of a quiet time` once `just test` passes; its `just check` runs together with group 2's (2.4), since every check queues behind other checkouts' runs on the machine lock.

## 2. Synthesize each phrase once

- [x] 2.1 Red: `e2e-test/src/speech_cache.rs` with unit tests `should_synthesize_a_phrase_once` and `should_keep_voice_rate_and_text_apart` against a cache with an injected synthesizer that counts its calls, and the cache's lookup as `todo!()`; `just test` fails on it.
- [x] 2.2 Green: the cache (key = FNV-1a of macOS build, voice, rate, format, text; directory `~/.cache/voicepuncher/speech` or `VOICEPUNCHER_SPEECH_CACHE`; write to a temporary file, then rename); the tests from 2.1 pass.
- [x] 2.3 `say` in `e2e-test/src/speech.rs` and `spoken` in `e2e-test/src/voice_commands.rs` go through the cache; `command-spotting/tests/common/mod.rs` gets the same cached synthesis with the same key, used by `engine_it.rs` and `placed_commands_it.rs`. A second `just test` run is faster than the first, and `engine_it`, `chunk_cuts_it` and the E2E harness unit tests pass.
- [x] 2.4 `AGENTS.md` § How to verify a change: one line on the speech cache and how to clear it. `just check` passes; commit `test: cache speech synthesized with say`.

## 3. E2E scenarios overlap

- [x] 3.1 `off_thread` in `e2e-test/tests/steps/mod.rs`; the blocking `When` steps of `voice_control.rs`, `voice_control/session.rs`, `voice_control/hands_free.rs`, `file_transcription.rs`, `speech_recognition.rs` and `text_refinement/` become `async` and run their blocking call through it. `just e2e-smoke` passes; its `cucumber_e2e` part takes clearly less than the 538 s baseline.
- [x] 3.2 `voice_forms_it` checks its forms four at a time (`map_concurrently`), `voice_samples_it` its sessions; both pass in `just e2e-smoke`.
- [x] 3.3 `just check` passes; commit `test(e2e): run scenarios and recorded forms four at a time`.
- [x] 3.4 Scenarios tagged `@accuracy` run alone (`which_scenario` in `e2e-test/tests/cucumber_e2e.rs`), and the hands-free sessions run alone (`@serial` on their rule): with real overlap 2 of 3 `just e2e-bench` runs failed with "command spotting service stopped answering", and once the synthesized hands-free session missed "cancel dictation" behind other scenarios' transcriptions. Hearing a scenario's two listening passes at once stalled the speech recognition service again and was dropped.

## 4. `just test` runs its binaries at once

- [x] 4.1 `_test` in `justfile`: build with `cargo test --no-run` for the three selections it runs today, take the executables from cargo's `Executable` lines, run every test executable at once in its package directory, print each binary's output whole under its `Running` line when it ends, then the five slowest binaries with their times; exit non-zero naming the failed binaries. `just test` passes; with failing tests (the Red run of 2.1) it exits non-zero, names the binaries and `kit run` shows each failure.
- [x] 4.2 `just check` passes; committed with group 5 as `chore(just): test binaries at once, services once per check, stage times`.

## 5. Check recipes

- [x] 5.1 The E2E recipes run their `cargo test` calls without `--release`; they keep `_build` for the release app binary. `just e2e-smoke`, `just e2e-quick` and `just refine-eval` pass; the hands-free scenarios pass (if they turn slow or flaky, `cucumber_e2e` alone keeps `--release`, as design decision 5 says).
- [x] 5.2 `_check` and `_full-check` run their stages through a timing function, start the services they did not find running once before the first E2E stage, stop them in an `EXIT` trap, and export `VOICEPUNCHER_E2E_SERVICES_HELD=1`, which makes the E2E recipes' own start and stop do nothing; `full-check` keeps `cargo clean`. Verify with the services stopped: `just check` starts each service once and leaves none running; with the services running it leaves them running.
- [x] 5.3 Both recipes end with one line per stage and a total, also when a stage fails. `just check` passes and prints them; committed with group 4 (4.2).

## 6. Measured times

- [x] 6.1 Time `just test` (warm and after an edit), `just check` and `just full-check` under the setup of `research/README.md` (fresh worktree, services running); add the results to `research/README.md` next to the baseline (`full-check` stops at the known seam failure, so the stages after it are timed on their own). Bars: `just test` ≤ 45 s, `just check` ≤ 6 min, `just full-check` ≤ 13 min; `full-check` passes bar the seam failures of `fix-dropped-phrase-at-word-pauses`.
- [x] 6.2 The bench scenario "Ordinary speech triggers no command" passes three `just e2e-bench` runs in a row.
- [x] 6.3 Replace the time estimates in the `justfile` recipe comments, `AGENTS.md` § How to verify a change and `openspec/config.yaml` `context` with the measured ones; commit `docs: measured check times`.

## 7. Version and manual checks

- [x] 7.1 No version bump: the proposal says no release; `app/Cargo.toml` keeps 0.10.0.
- [ ] 7.2 Manual check by the user (the command spotting client changed): `just run`, say "start dictation", a sentence, "stop dictation", then "press enter"; the log shows each command carried out and the text pasted.
