# Tasks

## 1. Red: reproduce the early read

- [ ] 1.1 Add `RECORD_LAG` (50 ms, doc comment: it stays so an early read fails every run) before the stub records a phrase selection in `app/tests/commandspotting_reconnect_it.rs`; verify 10 runs of `should_reach_for_the_service_again_after_it_closes_the_connection` each fail with an empty selection list
- [ ] 1.2 Run `just test` and verify it fails only in that test; commit alone as `test: reproduce reconnect test reading selections too early` with a message stating it is intentionally red

## 2. Green: wait for the stub's record

- [ ] 2.1 Add a helper that waits, within `RECORD_WITHIN` (2 s), until a stub connection's record meets a condition; make the reconnect test wait until the new connection holds a "press enter" selection, keeping its assertion; verify 20 runs in a row pass under CPU load
- [ ] 2.2 Make `should_send_the_service_that_is_back_the_speech_heard_while_it_was_away` wait the same way until the new connection holds more marked samples than the outage, keeping its assertions; verify 20 runs in a row pass under CPU load
- [ ] 2.3 Verify no bar moved: no app code changed, no assertion or existing timing constant changed, no retry in the diff
- [ ] 2.4 Run `just test` and `just check`, both pass; commit `fix(test): wait for the stub's record after reconnecting`

## 3. Version and manual checks

- [ ] 3.1 Verify `version` in `app/Cargo.toml` is unchanged: no bump and no release (proposal § Release)
- [ ] 3.2 Manual checks: none (hotkey, recording, overlay, paste and permissions are untouched)
