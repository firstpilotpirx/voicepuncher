# Tasks

## 1. A start ending in "please" is complete

- [ ] 1.1 Red: unit tests in `command-spotting/src/matcher/placed.rs` and `command-spotting/src/engine/confirming.rs` — "Start please." and "Start dictation, please." are a start at the head for the early head check and the first spotted check; a bare "Start." still is not; run `just test`, see them fail
- [ ] 1.2 Red: E2E scenarios "Start please followed by the dictation at once gives its cue at once" and "Start please said on its own gives its cue at once" in `e2e-test/tests/features/voice_control.feature` (`@seams`), reusing the existing start-cue steps
- [ ] 1.3 Green: the matcher's start-at-head rule takes a start whose command words end in "please" with nothing after it; `just test` green
- [ ] 1.4 `just test`, commit

## 2. A trigger in a short segment being heard asks its second hearing at once

- [ ] 2.1 Red: adapter integration test in `command-spotting/tests/placed_commands_it.rs` with a scripted hearing (first "Restocked.", steady clip "Restart!", each answered after a delay) — "restart" alone is reported one delay after the segment closes, not two; run it, see it fail
- [ ] 2.2 Red: adapter integration test in `placed_commands_it.rs` — a segment whose first hearing holds the command reports it once, the early second hearing dropped
- [ ] 2.3 Green: a rehear lane in `command-spotting/src/worker.rs`; the engine asks the steady clip at once when a trigger lies in a short segment whose hearing is pending, or when such a segment closes over a trigger; the segment's events wait for both answers (`command-spotting/src/engine/confirming.rs`); `just test` green
- [ ] 2.4 `just test`, commit

## 3. In standby a short utterance is heard at half the pause

- [ ] 3.1 Red: unit test in `command-spotting/src/engine/pause.rs` — the quiet run reports reaching half the minimum silence once, and forgets it on a louder frame
- [ ] 3.2 Red: adapter integration test in `placed_commands_it.rs` (scripted hearing with a delay) — "start" alone in standby is reported before one delay after the full pause ends
- [ ] 3.3 Red: adapter integration test in `placed_commands_it.rs` — "start", a dip shorter than a pause, then "stop", in standby, starts nothing; the early hearing is dropped
- [ ] 3.4 Red: adapter integration test in `placed_commands_it.rs` — during a dictation no utterance is heard before its pause ends
- [ ] 3.5 Green: `PauseWatch` reports the half-pause; the engine asks an early hearing of a short utterance in standby, uses its answer for the segment when the same quiet run closes it, drops it on new sound (`command-spotting/src/engine.rs`, `command-spotting/src/engine/confirming.rs`); `just test` green
- [ ] 3.6 `just test`, commit; `just check`

## 4. Release

- [ ] 4.1 Bump `version` in `app/Cargo.toml` to 0.24.4; `just test`, commit
- [ ] 4.2 Manual checks: none (no hotkey, recording, overlay, paste or permissions change; the app's voice control code is unchanged)
