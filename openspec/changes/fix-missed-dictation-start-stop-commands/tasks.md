# Tasks

## 1. Session 6 from the command history

- [x] 1.1 Assemble `~/.voicepuncher/voice-samples/session-6.wav` and `session-6/forms.jsonl` from the four entries of design D4 with a scratchpad script; verify: `just streaming-bench --recorded` lists `session-6/*` clips and `voice_forms` (`just test-one should_carry_out_every_recorded_way_of_saying_the_commands` with the service) runs them
- [x] 1.2 Commit nothing in the repository for this group (the recordings stay on the machine); note the layout in the PR body

## 2. A lone one-word command settles at the gate's pause (D1)

- [x] 2.1 Red: `decide` unit test from entry 684 — "stop" 0.818–0.918 s, `Moment::Decoding { now: 1.118, pause: None }` gives no command, `pause: Some(1.0)` gives "stop dictation"; `should_wait_for_a_one_word_form_to_settle` restated with a pause; committed alone as `test: reproduce a stop settled while the gate hears speech`, red on `just test-one`
- [x] 2.2 Green: `settled` requires the pause for a one-word command (D1); `just test-one` passes, `just static`, `just test`
- [x] 2.3 Commit `fix(command-spotting): settle a lone one-word command at the gate's pause`

## 3. A tail of silence before the final decode (D2, D3)

- [x] 3.1 Red: `StreamingEngine::session_with_gate` and a scripted gate in the streaming integration tests; a `needs_streaming_models::needs_voice_samples` test feeds the session-6 "старт" stretches with the gate's live hearing (D4 constants) in 10 ms frames and expects "start dictation" once each; committed alone as `test: reproduce a quiet start lost at the close`, red on `just test-one`
- [x] 3.2 Green: `CLOSE_SILENCE_SECS` fed before `finish` in `Utterance::close`, tuned to the smallest value that decodes both; `just test-one` passes, `just static`, `just test`
- [x] 3.3 Commit `fix(command-spotting): feed a tail of silence before the final decode`

## 4. The E2E scenarios and the measurements

- [x] 4.1 `voice_control.feature`: scenario "A Russian sentence heard as opening with stop is no command" (`@smoke @voice_samples`, real time through `run_clip`) under its rule; the quiet-start scenario is the D3 test (the gate cannot be replayed through the service); `kit feature-lint` and `spec_mirror` pass
- [ ] 4.2 Measure: `just streaming-bench --recorded` on sessions 1–6 loses no command recognized before; `e2e-cues` and the 0.3 s/0.6 s scenarios keep their bounds (`just check`)
- [x] 4.3 Commit `test(e2e): the session-6 scenarios of voice control`

## 5. Release

- [ ] 5.1 Bump `version` in `app/Cargo.toml` to the target version (`kit version-check`); `just test` passes
- [ ] 5.2 Commit `chore(release): bump version`
