## Why

The first launch of the bundle `dist/voicepuncher.app` (v0.37.3, macOS 27.0.1) through the GUI crashed about 10 s after the first dictation, on a key press, and logged a false warning about the recording limit. Split from `fix-first-gui-launch-bugs` (bugs 5–6).

1. **Crash on the first key press after a dictation.** SIGTRAP (`EXC_BREAKPOINT`) on the thread `rdev-hotkey`: rdev 0.5.3 `raw_callback` → `Keyboard::string_from_code` → `TSMGetInputSourceProperty` → `dispatch_assert_queue_fail`. macOS 27 requires the main queue for input-source calls, and rdev translates every key event into a string on its own thread, so any key press can kill the app. Report: `~/Library/Logs/DiagnosticReports/voicepuncher-2026-10-22-200853.ips`; listener `app/src/adapters/primary/ptt/rdev/mod.rs`.
2. **False recording-limit warning.** After a 5.7 s live recording the log warned "Recording reached the 600-second limit and was cut off at that point". `stop()` in `app/src/adapters/secondary/recording/cpal/mod.rs` checks `buffer.is_full()`, and the live watcher's final chunk calls `cut_at(len)` (`buffer.rs`), which lowers the buffer's limit to its length, so `is_full` holds.

## What Changes

- The hotkey listener no longer crashes on macOS 27: no key code is translated through the input-source API off the main thread.
- The recording-limit warning appears only when a recording really reached the limit.

Current stage: PoC. Target stage: PoC.
Scope: the hotkey listener and the recording-limit warning.
Acceptance bar: a test reproduces each bug red before its fix; on macOS 27 the app survives typing after several dictations, hotkey and voice started; a short live recording logs no limit warning, a recording cut at the limit still does; the manual run of README § Release checklist (hotkey) passes.
Out of scope: the permissions gate and the overlay (changes `fix-permissions-and-login-item`, `improve-recording-pill-look`).

## Capabilities

### Modified Capabilities

- `push-to-talk`: the hotkey listener survives every key press on macOS 27.
- `live-transcript`: the recording-limit warning only on a recording that reached the limit.

## Impact

- `app/src/adapters/primary/ptt/rdev/mod.rs` (or a listener replacing rdev), `app/Cargo.toml`, `Cargo.lock`
- `app/src/adapters/secondary/recording/cpal/mod.rs`, `buffer.rs`

## Open points

- Bug 1: replace rdev by our own `CGEventTap` listener, or use an rdev version or fork that skips the key-string lookup; proposed: our own event tap, as the hotkeys need only key codes and modifier flags.
- Bug 1: how to test the crash in CI, where macOS may be older; proposed: a test that the listener never calls the input-source API, plus the manual run on macOS 27.

## Release

SemVer: PATCH — bug fixes, no new capability.
Target version: 0.37.4 (the next PATCH above `main` when it merges).
