# Tasks

## 1. Red: a second run must wait for the pasteboard

- [x] 1.1 Add `app/tests/support/pasteboard_lock.rs` with a guard that opens the lock file without locking it, declared from `app/tests/textpasting_macos_it.rs`, and a test there: while one guard holds a temporary lock path, a second guard on that path is not taken within 200 ms and is taken within 2 s after the first is dropped; verify it fails because the second guard is taken at once
- [x] 1.2 Commit alone as `test: reproduce pasteboard tests not waiting for each other` with a message stating it is intentionally red

## 2. Green: the pasteboard tests take turns across checkouts

- [x] 2.1 Make the guard take an exclusive `flock` on `~/.cache/voicepuncher/pasteboard.lock` by default (directory created when missing), printing `waiting for the pasteboard lock <path>` to stderr when it is busy; the lock test passes
- [x] 2.2 Make every test of `textpasting_macos_it` take the guard first, before it saves the user's pasteboard, so it is released last; update the file's doc comment and the `pasteboard` test group comment in `.config/nextest.toml`
- [x] 2.3 Make the `@real_keyboard` step "the app pasted text into the focused app" (`app/tests/steps/push_to_talk.rs`) hold the guard until its paste is restored, the module declared from `app/tests/cucumber_integration.rs`
- [x] 2.4 Make `should_bring_back_a_copied_screenshot_after_the_paste` and `should_bring_back_copied_text_with_every_type_after_the_paste` wait, within `RESTORE_DEADLINE`, until every copied item is back before asserting (design D6)
- [x] 2.5 Verify the race is gone: run the `textpasting_macos_it` binary in two processes at once, 20 rounds each, all pass under load (before D6, 5 of 40 failed); verify no assertion, timing constant or time limit changed and no retry was added
- [x] 2.6 Run `just test`, passes; commit `fix(test): pasteboard tests take a machine-wide pasteboard lock`

## 3. Version and manual checks

- [x] 3.1 Verify `version` in `app/Cargo.toml` is unchanged: no bump and no release (proposal § Release)
- [x] 3.2 Manual checks: none (the app's hotkey, recording, overlay, paste and permissions are untouched)
