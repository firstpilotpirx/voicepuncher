# Tasks

## 1. A sync waits for a held segment moved to a later job (unit)

- [x] 1.1 Red: add `should_keep_a_sync_waiting_for_a_segment_whose_wait_moved_to_a_confirmation_asked_after_it` to `command-spotting/src/outbox/tests.rs`, run it, see it fail; commit it alone (`test: reproduce …`, intentionally red)
- [x] 1.2 Green: in `command-spotting/src/outbox.rs` keep the job a held segment was first held under on `transfer` and make `answered_syncs` wait while such a segment remains
- [x] 1.3 `just test`, commit

## 2. The command is reported before the sync answer (integration)

- [x] 2.1 Red: add to `command-spotting/tests/it/placed_commands.rs` a test where the first hearing of "clear text please" said alone in standby answers empty and late, a sync is sent right after the speech, and "clear all text" must come before the sync answer; run it, see it fail
- [x] 2.2 Red: add to `command-spotting/tests/it/placed_commands.rs` a test of "okay, clear" misheard at half the pause with a sync right after the speech, the command before the sync answer; run it, see it fail
- [x] 2.3 Green: make both pass with the outbox fix of group 1 (no new production code expected; otherwise fix in `command-spotting/src/outbox.rs` or `command-spotting/src/engine/confirming.rs`), without growing `heard_finely_misheard`
- [x] 2.4 `just test`, commit, then `just check` once

## 3. Version and manual checks

- [x] 3.1 Bump `version` in `app/Cargo.toml` to 0.37.3, `just test`, commit
- [x] 3.2 Manual checks: none (no hotkey, recording, overlay, paste or permissions change); the acceptance bar is met by the `e2e-regression` row `clear text please` passing in the next three `full-check` runs
