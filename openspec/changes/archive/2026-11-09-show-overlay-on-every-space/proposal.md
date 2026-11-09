## Why

The user switched to a second desktop (macOS Space), started a recording, and the recording ran but the overlay pill with its record sign and waveform did not show: it showed only on the first desktop. The overlay's windows are built to join every desktop, and freshly built ones do (checked on the user's machine: a fresh overlay window sits on every desktop, also on a desktop added later). The windows of the app that had run since the day before sat on one desktop only: the window server had dropped them from the desktop the user was on, for a reason macOS does not report (not a Dock restart, not a new desktop: both checked). The app never notices, so the overlay stays out of sight until the screens change and the windows are built again.

## What Changes

- Each time the overlay shows a state, and each time the user switches desktop while it shows one, the app checks that the pill is on the current desktop; when it is not, it builds the overlay's windows again (fresh windows join every desktop) and draws the state on them, and logs a warning saying so.

Current stage: PoC. Target stage: PoC.
Scope: the macOS overlay adapter `app/src/adapters/secondary/visualfeedback/macos/` (show path, rebuild, a desktop-switch observer next to the screen watcher in `app/src/adapters/screen_watcher.rs`), its component scenarios in `app/tests/features/component/visual_feedback.feature` and steps.
Acceptance bar: the new component scenarios pass (windows off the current desktop are built again and show the state, on show and on a desktop switch; a rebuild leaves no old overlay window); the existing visual-feedback scenarios stay green; `just check` passes; the user sees the overlay on a second desktop in the manual run.
Out of scope: the overlay's look, the pointer sign's placement, the permissions and settings windows, finding why the window server drops the windows, the old windows a rebuild leaves in the window server (design D4).
Interface: none (the look of the overlay is unchanged; only the desktop it shows on).

## Capabilities

### Modified Capabilities

- `visual-feedback`: the overlay shows on the desktop the user is on.

## Impact

- `app/src/adapters/secondary/visualfeedback/macos/` (`mod.rs`, `redraw.rs`, `window_set.rs`, `glass/mod.rs`), `app/src/adapters/screen_watcher.rs`, `app/tests/features/component/visual_feedback.feature`, `app/tests/steps/visual_feedback/overlay/`.

## Release

SemVer: PATCH — a bug fix: the overlay does not show on the current desktop.
Target version: 0.47.1.
