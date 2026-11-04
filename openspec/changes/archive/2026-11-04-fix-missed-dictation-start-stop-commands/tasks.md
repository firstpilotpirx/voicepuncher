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
- [x] 4.2 Measure: `just streaming-bench --recorded` on sessions 1–6 loses no command recognized before; `e2e-cues` and the 0.3 s/0.6 s scenarios keep their bounds (`just check`) — measured 2026-11-04: 47/53 clips right on the branch (main's binary: 73/89, its 36 extra clips the long forms the bench now leaves out, main heard them only through the early fire); the branch gains session-2/41, 4/15 and 6/0, 6/2, 6/4 (the session-6 misses), every other wrong clip is wrong on main too; the tails of chains (session-4/5, 4/7, 4/8) come and go between runs on both binaries (the English recognizer's late emits, out of scope); the 0.3 s/0.6 s scenarios hold, the doubled form inside speech moved to 1 s (the user's decision, see design D1)
- [x] 4.3 Commit `test(e2e): the session-6 scenarios of voice control`
- [x] 4.4 A doubled one-word form ends with its first word (`decide::finding`, test `should_report_a_doubled_command_at_once_ending_at_its_first_word`): the first `kit verify` lost "Coffee" of the sentence after the user's recorded "restart restart" (design D1)
- [x] 4.4a The doubled words are compared as the matcher reads them (`matcher::words`, test `should_end_a_doubled_command_written_with_a_capital_and_a_comma_at_its_first_word`): the seventh `kit verify` lost "Coffee" again, the English recognizer writing "Restart restart" with a capital
- [x] 4.5 The close waits for no decode (`Session::closing`, `Utterance::begin_close`, test `should_close_without_waiting_for_the_tail_of_silence_to_be_decoded`): the second `kit verify` failed `e2e-cues`, "press enter" after a sentence 1.44 s after the words with every core busy (design D2a)

## 5. Release

- [x] 5.1 Bump `version` in `app/Cargo.toml` to the target version (`kit version-check`); `just test` passes
- [x] 5.2 Commit `chore(release): bump version`
