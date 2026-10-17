# Tasks

## 1. Reproduce the lost end of a chain (Red)

- [ ] 1.1 Add to `command-spotting/tests/engine_it.rs` a scripted hearing that spreads each answer's words over the sound of its clip, and a test: "never mind cancel enter start" said while dictating, the window heard "Nevermind, cancel, enter style.", the segment "Nevermind, cancel, enter, start." gives "cancel dictation", "press enter", "start dictation" once each, in order; `just test` fails on the missing "start dictation"; commit as `test: reproduce the end of a chain lost after a window` (intentionally red)

## 2. Report the rest of a chain

- [ ] 2.1 Red: unit test in `command-spotting/src/matcher/placed.rs`: `anywhere` gives the word range of each command of a chain; `just test` fails to compile
- [ ] 2.2 Green: each command's word range in `Found` (design D1); `just test` passes
- [ ] 2.3 Green: `place` in `command-spotting/src/engine/confirming.rs` reports the commands whose words' middle lies after `reported_until`, placed by their own words, and nothing when none is left (design D1); `just test` passes, task 1.1 included
- [ ] 2.4 Run `voice_forms_it` on ports of its own with this branch's service, and with `detect-quiet-clear-all-text`'s `command-spotting/src/engine.rs` swapped in for the run only: every form passes; `kit verify after-group`, commit

## 3. Version and checks

- [ ] 3.1 Bump `version` in `app/Cargo.toml` to 0.15.2; `just test` passes
- [ ] 3.2 `just check` once; commit
- [ ] 3.3 Manual checks by the user: none (no change to hotkey, recording, overlay, paste or permissions)
