## Context

The stage journal and the run logs of 2026-10-20/21 (`~/.cache/voicepuncher/stage-times.jsonl`, `$TMPDIR/run-visible/*.log`) were read for every test of the proposal's list. Six related changes landed on `main` meanwhile (batch #259 and #252, #258): `steady-overlay-waveform-test` (the record sign drawn without its waveform), `match-late-confirmation-to-its-state`, `speed-up-short-start-cue`, `fast-confirmation-of-repeating-russian-speech`, `ordered-wait-for-busy-copies`, `steady-synthesized-hands-free-session`, and #253 the pasteboard lock. Every failure of the overlay's record-sign test and of the late-start tests in the logs ran on code without those fixes. What is left has three causes, each a test that measures time with the wall clock while the machine is shared:

1. `placed_commands_it` feeds speech on a wall-clock schedule and its scripted confirmations sleep a wall-clock delay. Under load the feeder falls behind and then catches up in a burst, so a 2 s confirmation lands later in the speech than 2 s and the chain loses its "stop" (seen 4 times on 2026-10-21 17:36–17:46). Its real-service tests feed the speech as fast as possible, so where in the speech an answer lands depends on the shared service's speed (`[]` instead of the command, 14:11 and 14:46).
2. `commandspotting_reconnect_it` ends its outages after fixed times: the long-outage test assumes the tenth failed attempt falls within 3.5 s (only attempt 1 was logged under load), the past-the-replay test waits 12 s from its start for the return.
3. `visualfeedback_macos_it` pumps the main run loop for fixed times: a loud level must raise a column within 0.4 s, a notice shown again must still show after 2 × 0.4 s.

## Goals / Non-Goals

**Goals:** these tests give the same result on an idle and on a loaded machine; no test deleted, no product bound loosened.

**Non-Goals:** the E2E scenarios under the machine lock (see Risks); the lock queue; the speed of a passing check.

## Decisions

1. **Confirmations in speech time.** A test clock (`command-spotting/tests/stream_clock`) counts the samples fed. A hearing wrapped in it asks its inner hearing (the script, or the real service with a deadline long enough for a shared service) at once and holds the answer until the speech has advanced by its delay. Before each frame the feeder waits until every confirmation the session sent is either answered and taken, or held for a later sample — so the stream never runs ahead of an answer that is due, whatever the threads' speed. The engine gets one read-only count for this, the confirmations sent to its worker and not yet taken back (`Session::confirmations_in_flight`); nothing else in the product changes. The wall-clock pacing of the feeder goes: the tests run faster. Alternative — longer delays or a lock group: rejected, they only make the race rarer.
2. **Outages ended by the client's own progress.** The long outage lasts until the client has logged its tenth failed attempt (bounded by the test's time limit), then the stub answers; "back within 5 s" is measured from the moment the stub answers again, as the requirement states, not from the test's start. A failure prints the client's log lines, so its cause is visible.
3. **The overlay read when it is drawn.** The loud-level test pumps until a column is taller than a dot (limit 2 s) instead of a fixed 0.4 s. The notice tests pump in small steps and decide from measured time: the notice shown again must be visible at a moment past the first notice's end and before its own, and hidden once its own time is over; pumps never assume the run loop ran on time.

## Risks / Trade-offs

- [The E2E scenarios under the lock (hands-free session, recorded starts, busy copies, Russian repetitions, the "silence" and "quiet restart" scenarios that failed together on 2026-10-21) cannot be run in this batch run, which is limited to `just test`] → the fixes of the six landed changes cover the first four; the batch's `full-check` measures the rest, and a scenario still failing there gets a fix change of its own.
- [The real `transcription_openai_it` tests failed with every copy silent for 13 s on the test port] → a service that does not answer is an outage, not a flake of the test; the batch's `full-check` shows whether it recurs.

## Test levels

| Scenario | Level |
|---|---|
| A chain is carried out the same on a busy machine | adapter integration (`placed_commands_it.rs`, scripted confirmations in speech time, the feeder starved on purpose) |
| A command is carried out once while confirmations come back slowly | adapter integration (`placed_commands_it.rs`, unchanged test, now in speech time) |
| A chain is carried out the same at any confirmation speed | adapter integration (`placed_commands_it.rs`, unchanged test, now in speech time) |
| A start confirmed after the dictation began stays a start | adapter integration (`placed_commands_it.rs`, unchanged test, now in speech time) |
| A start said right after a slowly confirmed stop stays a start | adapter integration (`placed_commands_it.rs`, unchanged test, now in speech time) |
