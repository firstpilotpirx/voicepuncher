## 1. Confirmations in speech time (placed_commands_it)

- [ ] 1.1 Red: add `should_carry_out_the_same_chain_on_a_busy_machine` — the chain with 2 s confirmations while the feeder is starved in bursts; it fails on the wall-clock hearing
- [ ] 1.2 Green: add `Session::confirmations_in_flight` (confirmations sent to the worker and not taken back) with a unit test in `worker.rs`
- [ ] 1.3 Green: add the speech-time clock and clocked hearing (`command-spotting/tests/stream_clock`), feed every scripted test through it, drop the wall-clock pacing
- [ ] 1.4 Green: run the real-service tests through the clocked hearing with a fixed latency in speech time and a long deadline
- [ ] 1.5 Commit: `fix: confirmations in speech time in placed_commands_it`

## 2. Outages ended by the client (commandspotting_reconnect_it)

- [ ] 2.1 The long outage ends once the tenth failed attempt is logged; the return is timed from when the stub answers again
- [ ] 2.2 The past-the-replay test times "back within 5 s" from the stub's return; failures print the client's log lines
- [ ] 2.3 Commit: `test: end reconnect outages by the client's progress`

## 3. The overlay read when drawn (visualfeedback_macos_it)

- [ ] 3.1 The loud-level test pumps until a column is taller than a dot
- [ ] 3.2 The notice-shown-again test decides from measured time in small pump steps
- [ ] 3.3 Commit: `test: read the overlay when it is drawn`

## 4. Release

- [ ] 4.1 Bump `version` in `app/Cargo.toml` to 0.32.1
- [ ] 4.2 Commit: `chore: bump version to 0.32.1`
