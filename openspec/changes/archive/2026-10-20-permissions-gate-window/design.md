# Design

## Context

`Preflight::run(probe)` (`core/application/preflight.rs`) fails with the first missing permission and `main.rs` exits 2. `MacOsPermissionProbe` checks accessibility with `AXIsProcessTrustedWithOptions(null)`, the microphone by opening a cpal input stream, input monitoring by creating a dummy event tap, and names the parent process with `ps`. `docs/architecture/context.md` documents this flow. `restartable-app-runtime` provides runtime states and reload; `menu-bar-presence` the menu; `app-bundle-and-dmg` the bundle identity. The user's words: check always, guide with buttons to the exact page.

## Decisions

**D1 — A gate with three-valued statuses, not a preflight with a boolean.** `PermissionStatus::{Granted, Denied, NotDetermined}` lets the window offer the right action (request versus open settings) and tells revocation from "never asked". The gate is a pure use case over the probe port; the window and the runtime read its report.

**D2 — Status APIs that do not touch devices.** `AVCaptureDevice.authorizationStatus` instead of opening a cpal stream (which prompted as a side effect and lit the microphone indicator); `IOHIDCheckAccess` instead of creating an event tap (which could not report NotDetermined). Requests are explicit and separate.

**D3 — IOHID through `extern "C"`, AVFoundation through `objc2` messaging and `block2`.** Two C functions need no crate (the code base already declares `AXIsProcessTrustedWithOptions` this way); the two AVFoundation class methods are sent with `msg_send!` on `AVCaptureDevice` and the `AVMediaTypeAudio` constant declared as an extern static, the completion block made with `block2` (pinned exactly, already in the lock file through `objc2`). `objc2-av-foundation` was rejected: a whole framework binding for two calls.

**D4 — Polling, not notifications.** macOS offers no public notification for TCC changes; three cheap calls every 5 s while running (2 s while waiting) are invisible in CPU and catch revocation within the acceptance bar.

**D5 — The runtime owns the gate's consequences.** Waiting for permissions is a runtime state; the runtime starts the pipeline when the gate opens and stops it (cancelling a dictation with a notice) when it closes. The window only displays and requests; it never starts the pipeline itself.

**D6 — Primary adapter window built in code.** `adapters/primary/permissions/macos/` builds the window with AppKit stack views (no Interface Builder, no SwiftUI), the same technique as the overlay and the status item, with pure mapping functions unit-tested.

**D7 — The responsible application is still named.** An unbundled binary's permissions belong to its parent; the probe reports it and the rows say "grant to iTerm2". In a bundle `responsible_app` is None and the rows name voicepuncher.

**D8 — The open points of the proposal.** The window floats above other windows but takes focus only when it first shows (seen, never stealing focus on a poll); `Continue` stays as the explicit close and the window does not close by itself (predictable); the running poll is 5 s (no public TCC notification); the parent application stays named for an unbundled run (developers run `cargo run` daily).

**D9 — The runtime state is Waiting for permissions with the missing list.** It sits beside Starting, Ready, Restarting, Stopped and Failed in the runtime's observable state; the menu bar and the window observe it like any other state, so neither polls the gate on its own: the runtime's permissions thread is the only poller, and the window's 2 s refresh asks the runtime for a fresh report.

## Risks / Trade-offs

- [The AX prompt option adds the app to the list unticked and the user believes it is done] → the row keeps saying Not granted with Open System Settings until the status call says Granted.
- [`IOHIDRequestAccess` returns at once; the dialog is asynchronous] → the status is polled; the row shows Checking… for up to 2 s.
- [A window at floating level annoys while the user grants in System Settings] → it does not steal focus after the first show; it can be closed and reopened from the menu.

## Test levels

| Scenario | Level |
|---|---|
| Start with a permission missing (app-runtime) | component `app_runtime.feature` |
| The gate opens while waiting | component `app_runtime.feature` |
| A revocation while Ready | component `app_runtime.feature` |
| A missing permission at launch | unit tests in the menu bar mapping |
| Permissions while every permission is granted | unit tests in the menu bar mapping; manual run |
| Launch with every permission granted | component `permissions_gate.feature` |
| Launch with a permission missing | component `permissions_gate.feature` |
| The last grant starts dictation | component `permissions_gate.feature` |
| Open System Settings lands on the permission's page | adapter integration `permissions_macos_it.rs` (deep links); manual run (the pane) |
| Request shows the system prompt | unit tests in `mapping.rs` (button, Checking…); manual run (the prompt) |
| A granted row has no button | unit tests in `mapping.rs` |
| A permission never requested | component `permissions_gate.feature`; unit tests in `mapping.rs`; manual run (indicator) |
| A permission denied | component `permissions_gate.feature`; unit tests in `mapping.rs` |
| Accessibility revoked while idle | component `permissions_gate.feature`; manual run |
| Microphone revoked during a recording | component `permissions_gate.feature` |
| Granting a revoked permission again resumes dictation | component `permissions_gate.feature` |
| Run from a terminal | unit tests in `mapping.rs`; adapter integration `permissions_macos_it.rs` |
| Run from the bundle | unit tests in `mapping.rs` |
| A grant in System Settings shows up in the open window | manual run |
| Continue waits for every permission | unit tests in `mapping.rs` |
| Continue closes the window once all are granted | manual run |
| A change is logged | unit tests in `permissions_gate.rs`; component `permissions_gate.feature` |
| File transcription without permissions | component `permissions_gate.feature` and `file_transcription.feature` (existing) |
| The window's look against the mockups in light and dark | manual run |
