# Design

## Context

`Preflight::run(probe)` (`core/application/preflight.rs`) fails with the first missing permission and `main.rs` exits 2. `MacOsPermissionProbe` checks accessibility with `AXIsProcessTrustedWithOptions(null)`, the microphone by opening a cpal input stream, input monitoring by creating a dummy event tap, and names the parent process with `ps`. `docs/architecture/context.md` documents this flow. `restartable-app-runtime` provides runtime states and reload; `menu-bar-presence` the menu; `app-bundle-and-dmg` the bundle identity. The user's words: check always, guide with buttons to the exact page.

## Decisions

**D1 — A gate with three-valued statuses, not a preflight with a boolean.** `PermissionStatus::{Granted, Denied, NotDetermined}` lets the window offer the right action (request versus open settings) and tells revocation from "never asked". The gate is a pure use case over the probe port; the window and the runtime read its report.

**D2 — Status APIs that do not touch devices.** `AVCaptureDevice.authorizationStatus` instead of opening a cpal stream (which prompted as a side effect and lit the microphone indicator); `IOHIDCheckAccess` instead of creating an event tap (which could not report NotDetermined). Requests are explicit and separate.

**D3 — IOHID through `extern "C"`, AVFoundation through `objc2-av-foundation`.** Two C functions need no crate (the code base already declares `AXIsProcessTrustedWithOptions` this way); the AVFoundation call needs Objective-C messaging and a block, which the objc2 family provides; both are pinned exactly.

**D4 — Polling, not notifications.** macOS offers no public notification for TCC changes; three cheap calls every 5 s while running (2 s while waiting) are invisible in CPU and catch revocation within the acceptance bar.

**D5 — The runtime owns the gate's consequences.** Waiting for permissions is a runtime state; the runtime starts the pipeline when the gate opens and stops it (cancelling a dictation with a notice) when it closes. The window only displays and requests; it never starts the pipeline itself.

**D6 — Primary adapter window built in code.** `adapters/primary/permissions/macos/` builds the window with AppKit stack views (no Interface Builder, no SwiftUI), the same technique as the overlay and the status item, with pure mapping functions unit-tested.

**D7 — The responsible application is still named.** An unbundled binary's permissions belong to its parent; the probe reports it and the rows say "grant to iTerm2". In a bundle `responsible_app` is None and the rows name voicepuncher.

## Risks / Trade-offs

- [The AX prompt option adds the app to the list unticked and the user believes it is done] → the row keeps saying Not granted with Open System Settings until the status call says Granted.
- [`IOHIDRequestAccess` returns at once; the dialog is asynchronous] → the status is polled; the row shows Checking… for up to 2 s.
- [A window at floating level annoys while the user grants in System Settings] → it does not steal focus after the first show; it can be closed and reopened from the menu.

## Test levels

| Scenario | Level |
|---|---|
| Report, missing list, all granted, responsible app | unit tests in `permissions_gate.rs` (mock probe) |
| Status → badge, permission → texts, request versus settings button | unit tests in `permissions/macos/mapping.rs` |
| The pipeline starts only when every permission is granted; the last grant starts it | component `permissions_gate.feature` (mock probe driven by steps, runtime with mock adapters) |
| A permission revoked while running stops dictation and reopens the window | component `permissions_gate.feature` |
| Not asked yet differs from denied | component `permissions_gate.feature` |
| File mode needs no permission | component `file_transcription.feature` (existing) + `permissions_gate.feature` |
| Statuses are three-valued on macOS; deep links well formed; parent named when unbundled | adapter integration `permissions_macos_it.rs` |
| The window's look, prompts, panes, revocation in real System Settings | manual run by the user |
