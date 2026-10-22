## Why

Voice commands are lost when the command confirmations run slow under machine load, and one retry of the dictation transcriber is skipped under the same load.

Evidence:

- The e2e-smoke light phase on `feat/command-spotting-stall-handling` 5c19ed3b (2026-10-22 06:58–07:02) ran beside four other sessions' `just test` runs. Three voice control tests lost every command:
  - `placed_commands_it should_hear_a_stop_right_after_the_last_dictated_word` got no command;
  - `voice_forms_it` had 1 failure of 82: session-4 at 26.19 s, "that is all stop enter start", carried out `[]`;
  - the scenario "The user's quiet restart after any pause before it" recognized `[]`.
- The command spotting service log of that run (`voicepuncher-command-spotting-27082.log`) shows the message "confirmation cut short after 1000ms on copy …: still decoding, nothing heard" 47 times. It comes from `cut_short` in `command-spotting/src/confirm.rs`, on `main` since v0.32.0. The log shows no queue refusal and no audio drop, so the stall handling of 0.37.0 did not cause the loss.
- The batch 7 e2e-regression also lost "press enter, start dictation" and a quiet "start dictation", without that branch.
- `transcription_openai_it should_transcribe_by_a_new_attempt_when_the_first_request_hangs` failed once under the load of a full `just test`: 1 request in 0.5 s, because the first request went to the copy that answers instead of the one that hangs. It passed 232 of 232 runs alone. Suspected cause: `TcpProbe` in `copy-pool` gives a connection only 200 ms (`CONNECT_TIMEOUT`), so under load a probe misses a copy and the pool counts it as down.

## What Changes

- A command said while the speech recognition service for commands answers slowly under load is still carried out, in order, instead of being dropped by a confirmation cut short while the copy is still decoding.
- A copy of a speech recognition service that accepts connections is not counted as down because one probe was slow under load, so a retry after a hung request still goes to another copy.

Current stage: PoC. Target stage: PoC.
Scope: the confirmation of spotted commands in the command spotting service under load, and the copy probe of `copy-pool`.
Acceptance bar: the three voice control tests above and the transcriber retry test pass while other sessions' `just test` runs load the machine, with no time limit raised and no bar loosened; `just check` passes.
Out of scope: the stall and backlog handling of `command-spotting-stall-handling` (its job cap and audio drop did not fire in these runs); model choice.

## Capabilities

### Modified Capabilities

- `voice-control`: a command whose confirmation is slow under load is not lost.
- `local-services`: a copy is not counted as down after one slow probe.

## Impact

- `command-spotting/src/confirm.rs` (`cut_short`), maybe the engine's confirmation scheduling.
- `copy-pool/src/probe.rs` (`CONNECT_TIMEOUT`), `copy-pool/src/pool.rs`.
- Tests: `command-spotting/tests/placed_commands_it.rs`, `app/tests/voice_forms_it.rs`, `e2e-test/tests/features/voice_control.feature`, `app/tests/transcription_openai_it.rs`.

## Open points

- Whether a cut-short confirmation should be retried, waited for longer while the copy keeps decoding, or the spotted command kept. Proposed: design decides from the logs of a loaded run; no deadline raised.
- Whether a copy should count as down only after several missed probes in a row. Proposed: yes, two in a row.
- How to reproduce the load in a test without other sessions. Proposed: a clocked slow-hearing variant in `placed_commands_it`, plus a CPU-load helper for the copy probe test.
- Target version. Proposed: 0.37.1, the PATCH above 0.37.0, which `command-spotting-stall-handling` brings.

## Release

SemVer: PATCH — a bug fix: commands and retries lost under load.
Target version: 0.37.1.
