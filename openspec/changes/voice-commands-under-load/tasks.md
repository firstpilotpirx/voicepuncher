# Tasks

## 1. The decoding budget follows the pace of the recent confirmations

- [ ] 1.1 Red: add `command-spotting/tests/scripted_copy/mod.rs` (an HTTP copy answering from `ScriptedHearing` after a wall-time decoding) and the three component tests of the `voice-control` delta in `placed_commands_it.rs` on a copy decoding 1.2 s; run them, see the commands lost
- [ ] 1.2 Red: add to `confirm/tests.rs` the scenarios "decoding slowly under load is heard in full" and "the budget never passes the deadline" on the fake live copy; run them, see them fail
- [ ] 1.3 Red: add `confirm/pace.rs` unit tests for the budget rule (1 s floor, twice the slowest of the last 4, capped at the deadline, a cut-short counted with its budget); run them, see them fail
- [ ] 1.4 Green: implement `Pace` in `confirm/pace.rs` and use it in `UtteranceTranscriber::transcribe` for the budget and the outcomes, keeping `transcribe` at its ratcheted complexity; adjust the GIVEN of `should_hear_nothing_when_a_live_copy_decodes_past_the_budget` to quick recent answers; `cargo nextest run -p voicepuncher-command-spotting` passes
- [ ] 1.5 `just test` passes, commit `fix(command-spotting): let the decoding budget follow the pace of the confirmations`

## 2. A missed probe is tried again at once

- [ ] 2.1 Red: add to `copy-pool/src/pool/tests.rs` a scripted probe that misses a port once, and the tests "a copy missed by one slow probe stays, probed twice" and "a copy missed by two probes in a row leaves"; run them, see them fail
- [ ] 2.2 Green: make `Inner::probe_round` in `copy-pool/src/pool.rs` probe a missed port once more in the same round; `cargo nextest run -p voicepuncher-copy-pool` and `app/tests/transcription_openai_it.rs` pass
- [ ] 2.3 `just test` passes, commit `fix(copy-pool): drop a copy only after two missed probes in a row`

## 3. Version and checks

- [ ] 3.1 Record the measured time of `placed_commands_it` against its limit in `.config/nextest.toml` (raise nothing; a suite past half its limit gets a new limit in this change per AGENTS.md)
- [ ] 3.2 Bump `version` in `app/Cargo.toml` to 0.37.1; `just test` and one `just check` pass; commit `chore(release): 0.37.1`
- [ ] 3.3 Manual checks by the user: none (no hotkey, recording, overlay, paste or permission change)
