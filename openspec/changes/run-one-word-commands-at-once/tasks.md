## 1. Baseline

- [ ] 1.1 Run `just streaming-bench --recorded` on `main`'s code in real time and record the recognized count and the delay median/p90 of commands said right after speech in design.md § D4
- [ ] 1.2 Commit

## 2. A command word counts anywhere at once (D1, D3)

- [ ] 2.1 Red: unit tests in `command-spotting/src/streaming/decide/tests.rs` — a one-word command ending the words is reported once decoded 0.2 s past it while the gate still hears speech; one with speech after it is reported the same way; "stop the car" reports "stop"; "start" that may grow into "restart" still waits; a repeat within 0.6 s is not reported again
- [ ] 2.2 Green: fold `alone` and `ending` into one at-once path in `command-spotting/src/streaming/decide/mod.rs`, drop the pause clause of `settled`
- [ ] 2.3 Red: session test in `command-spotting/src/streaming/session/tests/` — "stop" after more than 10 s of speech, the gate still hearing speech, is reported within 0.6 s of its end
- [ ] 2.4 Green: whatever the session test still needs in `command-spotting/src/streaming/session/utterance.rs`
- [ ] 2.5 `just test`, commit

## 3. An English-only command where Russian heard another word keeps the old rule (D2)

- [ ] 3.1 Red: unit test — the English recognizer's "stop" heading a sentence that goes on, while the Russian recognizer holds another word over its span, is no command; at the gate's pause right after it, it counts
- [ ] 3.2 Green: pass the Russian recognizer's last words over the finding's span to the decision for the English recognizer in `command-spotting/src/streaming/session/utterance.rs` and `decide/mod.rs`
- [ ] 3.3 `just test`, commit

## 4. Scenarios mirror the new requirements

- [ ] 4.1 Rename the six `Rule:` headings in `e2e-test/tests/features/voice_control.feature` and `app/tests/features/component/voice_control.feature` to the added requirements
- [ ] 4.2 Red: flip the scenarios in those files — a command word inside a sentence counts while listening, a stop word inside a dictated sentence ends the dictation at once, Enter followed by speech presses Return, a paste word inside a sentence pastes at once, a long form counts by its command word, stop opening a sentence counts at once; add press enter after a sentence without waiting for the pause, enter enter presses Return once, stop after a long dictation without a pause; step definitions in `e2e-test/tests/steps/voice_control/` and `app/tests/steps/`
- [ ] 4.3 Green: run them with `just test-one` (component) and `just e2e-smoke` (E2E); fix what fails
- [ ] 4.4 `just test`, commit

## 5. Measure

- [ ] 5.1 Run `just streaming-bench --recorded` on the branch in real time; record in design.md § D4 that every command is still recognized once and the delay median of commands after speech is at most 0.4 s
- [ ] 5.2 Commit

## 6. Release

- [ ] 6.1 Bump `version` in `app/Cargo.toml` to 0.48.0
- [ ] 6.2 `just check`, commit
- [ ] 6.3 Manual checks: none (the change is inside the command spotting service; hotkey, recording, overlay, paste and permissions are unchanged)
