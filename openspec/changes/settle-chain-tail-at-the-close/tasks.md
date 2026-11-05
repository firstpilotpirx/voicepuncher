## 1. The tail of a chain (D1, D2)

- [ ] 1.1 Red: `decide` unit tests in `command-spotting/src/streaming/decide/tests.rs` from the session-4/5 trace: with the bound at the "enter" end, "start dictation" starting at the "start" word; with the bound at the "start" end, nothing (`just test-one` fails on the first)
- [ ] 1.2 Red: session test in `command-spotting/src/streaming/session/tests/` (new file `chain_tail.rs`): the English recognizer says "stop enter start" with its lag, the Russian hears "стоп энтер", 20 ms frames; expects `[stop dictation, press enter]` then `[start dictation]` (`just test-one` fails)
- [ ] 1.3 Green: `chain_tail` in `decide/mod.rs`, called after `placed`; `just test-one` on both green, `just static`
- [ ] 1.4 `just test` for the group; `kit ship -m "fix(command-spotting): report the tail of a chain decoded after its first commands"`

## 2. The user's recorded chains (D3)

- [ ] 2.1 Red: the scenario "The user's recorded chains keep their last command" (`@smoke @voice_samples`) under the rule "A chain of commands is carried out in order" in `e2e-test/tests/features/voice_control.feature`, with its step in `e2e-test/tests/steps/voice_control/` playing each recorded chain ending in "start" in real time while dictating and checking its label's commands in order; `kit feature-lint`
- [ ] 2.2 Run it (`just e2e-smoke` filtered, or the voice smoke part) green on this branch; `just test`; `kit ship -m "test(e2e): the user's recorded chains keep their last command"`

## 3. Measurement and version (D4)

- [ ] 3.1 `just streaming-bench --recorded ~/.voicepuncher/voice-samples --only session-4` three times and every session once, after D1; fill the table of design.md D4
- [ ] 3.2 `kit version-check settle-chain-tail-at-the-close`; bump `version` in `app/Cargo.toml` to the version it names (0.46.3 unless a peer took it) and the proposal's target with it
- [ ] 3.3 `just test`; `kit ship -m "chore(release): bump version to 0.46.3"`
