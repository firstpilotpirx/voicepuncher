## Why

The overlay's thread safety rests on convention (audit of 2026-10-22): `unsafe impl Send for MacosVisualIndicator` (`app/src/adapters/secondary/visualfeedback/macos/mod.rs:534`) has no justification; `WindowSet` and the pointer sign claim `Send + Sync` (`window_set.rs:136–137`, `pointer_sign.rs:44–45`) on "all access is on the main queue", enforced only by a `debug_assert!` in `Drop`; the test accessors (`mod.rs:160–240`, `pointer_sign.rs:62–89`) read `NSWindow` without a marker. The indicator sits behind `Arc<Mutex<Box<dyn VisualIndicator>>>` and is driven from pipeline threads; correctness comes from every mutation going through `dispatch_to_main`, which the type system does not guarantee — a refactor that calls one AppKit getter off the main thread compiles silently. Twelve `lock().unwrap()` in the overlay (`mod.rs:143–378`, `pill_states.rs:83, 247`, `failure_notice.rs:30–41`, `pointer_sign.rs:91–114`) turn a panic in one dispatched closure into a poisoned lock that brings the next signal down, while `menubar/macos/indicator.rs:82–87` shows the poison-tolerant pattern. The clipboard restore after a paste runs on a detached thread (`textpasting/macos/mod.rs:97–101`); quitting before `restore_delay_ms` loses the user's clipboard; the same thread-per-event pattern is in `indicator.rs:108–118` and `pointer_sign.rs:112–118`. `afplay` children are never reaped (`soundplaying/macos/mod.rs:84–89`); service output copy threads are never joined (`process/mod.rs:54–70`).

## What Changes

- The AppKit-owning types are not `Send`/`Sync`: they live on the main thread only, reached through a handle that is `Send` because it carries nothing but a way to dispatch a closure to the main queue (`main_queue.rs`); the `VisualIndicator` port implementation is that handle. Any off-main access fails to compile.
- Overlay locks tolerate poisoning as the menu bar's do, or the state moves to the main thread where no lock is needed.
- Timers and delayed work (clipboard restore, notice expiry, pointer sign) are owned: cancelled or joined at quit, so a quit after a paste still restores the clipboard; a restore that cannot run before quit runs at once.
- Sound playback reaps its child; service output threads are joined at stop.
- The three `unsafe impl` leave the allowlist of `enforce-unsafe-safety-comments`.

Current stage: PoC. Target stage: PoC.
Scope: `visualfeedback/macos/*`, `visualfeedback/pointer_sign.rs`, `main_queue.rs`, `textpasting/macos`, `soundplaying/macos`, `localservices/process`, the integration tests of each.
Acceptance bar: no `unsafe impl Send`/`Sync` in `app/`; a test that calls an AppKit getter off the main thread does not compile (a `compile_fail` doctest or trybuild case); quit right after a paste restores the clipboard (integration test); no zombie after a sound.
Out of scope: the overlay's look and states; the menu bar.

## Capabilities

### Modified Capabilities

- `text-insertion`: the clipboard is restored even when the app quits before the delay.

## Impact

- `app/src/adapters/secondary/visualfeedback/**`, `app/src/adapters/main_queue.rs`, `app/src/runtime/mod.rs` (`SharedVisualIndicator`), `app/src/adapters/secondary/{textpasting,soundplaying,localservices/process}/`, `app/tests/{visualfeedback,textpasting,soundplaying,localservices_process}_*_it.rs`.

## Open points

- Whether `SharedVisualIndicator`'s `Mutex` stays: decided it stays — dropping it needs `VisualIndicator: Sync` for every implementor, beyond this change's scope; the lock is already poison-tolerant.

## Release

SemVer: PATCH — the clipboard restore at quit is a visible fix.
Target version: 0.37.1 (next PATCH above `command-spotting-stall-handling`'s 0.37.0, which lands in the same batch; `kit version-check` corrects it).
