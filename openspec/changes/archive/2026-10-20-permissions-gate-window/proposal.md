## Why

voicepuncher needs three macOS permissions: Microphone (to record), Accessibility (to paste with a synthesized ⌘V and to read the focused text field), Input Monitoring (to notice the hotkey in any app). Today `Preflight` checks them once at start through `PermissionProbe` (`AXIsProcessTrustedWithOptions`, a cpal input stream, a dummy `CGEventTapCreate`) and, when one is missing, logs `permission denied for X; grant X to PARENT` and exits with code 2. In a terminal that is readable; in a bundle launched from the Finder it is an app that silently dies. The user asked for a GUI that checks the permissions at every launch and while the app runs — not only on the first launch — tells which ones are missing, and takes the user to the exact System Settings page with a button.

A second reason: the permission identity changes. Until now the permissions were granted to the terminal that ran the binary (the probe names the parent process for that reason). With the signed bundle (`app-bundle-and-dmg`) they are granted to voicepuncher itself; the window must say the right name in both cases, because developers keep running `cargo run` from a terminal.

## What Changes

- A Permissions window replaces the fail-fast preflight. It opens by itself at launch when any of the three permissions is missing, and from the menu (`Permissions…` under the status line, `menu-bar-presence`) at any time. The dictation pipeline starts only when all three are granted; until then the runtime state is Waiting for permissions, the menu bar icon shows the waiting icon and the status line says `Waiting for permissions — open Permissions…`.
- The window (exact look in `mockups/design-spec.md`, made with Claude Design before the proposal gate): title `Permissions`; a heading `voicepuncher needs three permissions`; one row per permission with an icon (SF Symbols `mic`, `accessibility`, `keyboard`), the name, one line of why (`to hear your dictation`; `to paste the text into the app you are typing in`; `to notice the hotkey in any app`), a status badge (green `Granted`; red `Not granted`; grey `Not asked yet`; `Checking…` while a request is pending) and one button: `Request…` when not asked yet (shows the system prompt), `Open System Settings` when not granted (opens the exact pane: Microphone, Accessibility or Input Monitoring in Privacy & Security). A footer: `voicepuncher checks again by itself; no restart is needed.` and the buttons `Quit` and `Continue` (`Continue` enabled only when all three are granted; it closes the window; the pipeline is already running by then).
- When the app runs from a terminal (not bundled), the window names the app that must be granted in one line under the heading: `voicepuncher runs from iTerm2 — grant the permissions to iTerm2.` (the probe's responsible app), because macOS attributes the permissions of an unbundled binary to the terminal that runs it. In the bundle the window names no other app.
- The checks run at launch, whenever the app becomes active or the window becomes key, every 2 s while the window is open, and every 5 s while the pipeline runs. A permission revoked while the app runs (the user removed it in System Settings) stops the pipeline (a dictation in progress is cancelled with the cancel sound and a failure notice `Microphone permission was revoked`), reopens the window with that row red, and the icon shows the waiting state; granting it again starts the pipeline without a relaunch (`restartable-app-runtime`).
- Requesting: Microphone — the system prompt through AVFoundation (`AVCaptureDevice requestAccessForMediaType:`); Accessibility — `AXIsProcessTrustedWithOptions` with the prompt option (the system dialog that offers to open System Settings and adds the app to the Accessibility list); Input Monitoring — `IOHIDRequestAccess(kIOHIDRequestTypeListenEvent)` (the system dialog that adds the app to the Input Monitoring list). Checking: Microphone — `AVCaptureDevice authorizationStatusForMediaType:` (Authorized, Denied, Restricted, NotDetermined; no device is opened); Accessibility — `AXIsProcessTrustedWithOptions` without prompt; Input Monitoring — `IOHIDCheckAccess(kIOHIDRequestTypeListenEvent)` (Granted, Denied, Unknown), replacing the dummy event tap, which had side effects and could not tell "not asked yet" from "denied".
- Deep links (opened with `NSWorkspace openURL`): `x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone`, `…?Privacy_Accessibility`, `…?Privacy_ListenEvent`.
- Exit code 2 for a missing permission is gone in hotkey mode (the window is the answer); file mode (`--input-file`) needs no permission and is untouched. The log says `permissions: microphone granted, accessibility not granted, input monitoring not asked yet` at every change.
- Docs: `docs/architecture/context.md` (the permission flow and sequence diagram: Preflight → exit 2 becomes the gate and the window), `docs/ubiquitous-language.md` (`Preflight` retired; Permissions gate, Permission status, Permissions window), `README.md` § macOS permissions (the window, the panes, the developer case), `docs/project-map.md`. Tech matrix (agent-kit PR, section "macOS Desktop"): permission APIs — AVFoundation authorization status and request (`msg_send!` on `AVCaptureDevice` with a `block2` completion block, no `objc2-av-foundation`), `AXIsProcessTrustedWithOptions` with prompt, IOHID `IOHIDCheckAccess` / `IOHIDRequestAccess` for Input Monitoring (extern "C", no crate), System Settings deep links `x-apple.systempreferences:…`; `docs/architecture/overview.md` the permission row.

Current stage: PoC. Target stage: Prototype (a user is guided to every permission from inside the app).
Scope: the gate use case and its three-valued probe; the window; requests, deep links, re-checks; revocation handling; runtime integration; docs; mockups.
Acceptance bar: on a Mac where the app has no permissions, the first launch shows the window with three red or grey rows; each `Request…` shows the right system prompt and each `Open System Settings` lands on the right pane; the pipeline starts by itself when the last permission is granted; revoking Accessibility while the app runs reopens the window within 5 s and stops dictation; the window's pictures match the approved mockups in light and dark appearance; the user pastes a healthy log.
Out of scope: the setup wizard removal and README rewrite (`retire-cli-setup-wizard`); the Screen Recording or Automation permissions (not needed); a first-run tutorial beyond this window.

## Capabilities

### New Capabilities

- `permissions-gate`: the three permissions are checked at every launch and while the app runs; the user is told which are missing and taken to the exact System Settings page; dictation starts only with all three and stops when one is revoked.

### Modified Capabilities

- `menu-bar-presence`: a `Permissions…` item under the status line; the waiting state while permissions are missing.
- `app-runtime`: a new state, Waiting for permissions, between Starting and Ready; the pipeline starts when the gate opens and stops when it closes.

## Requirements to specify

1. The pipeline starts only when every permission is granted: at launch with a permission missing the window opens and the state is Waiting for permissions; when the last one is granted the pipeline starts without a relaunch.
2. Each missing permission is named with its page: a row shows the permission, why it is needed, its status, and a button that opens the exact pane or shows the system request.
3. A permission revoked while the app runs stops dictation: within 5 s the pipeline stops, a recording in progress is cancelled with a notice, and the window opens with that row marked.
4. The responsible app is named when running from a terminal: an unbundled run names the parent application to grant to; a bundled run names voicepuncher.
5. The window can be opened at any time from the menu and shows the current statuses, re-checked every 2 s while open.
6. Not asked yet is different from denied: a permission never requested shows `Request…`; a denied one shows `Open System Settings`.
7. File mode needs no permission: `--input-file` runs without any check or window.

## Implementation guide

1. **Mockups first.** `mockups/design-spec.md`; PNGs in `mockups/states/`; shown with the proposal at stage 3.
2. **Port.** `core/application/permission_probe.rs` (with `PermissionStatus { Granted, Denied, NotDetermined }` in `core/domain/permission.rs`): `fn status(&self, p: Permission) -> PermissionStatus`; `fn request(&self, p: Permission)` (fire and forget, the status is polled); `fn settings_url(&self, p: Permission) -> String`; `fn responsible_app(&self) -> Option<String>` (parent name when unbundled, None in a bundle: detect with `NSBundle mainBundle bundleIdentifier` being set, or `CFBundleGetMainBundle` info dictionary). `Permission { Microphone, Accessibility, InputMonitoring }` is a domain enum (`core/domain/permission.rs`).
3. **Gate use case.** `core/application/permissions_gate.rs`: `PermissionsGate::new(probe)`, `report() -> PermissionReport` (three statuses + responsible app), `all_granted()`, `missing() -> Vec<Permission>`; pure, unit-tested with a mock probe. `Preflight` is deleted (its tests move to the gate).
4. **macOS probe adapter.** `adapters/secondary/permissions/macos/mod.rs`: microphone through `msg_send!` on `AVCaptureDevice` (`authorizationStatusForMediaType:`, `requestAccessForMediaType:completionHandler:`) and `block2 = "=0.6.2"` (pinned exactly) for the completion block, no `objc2-av-foundation` (design D3); accessibility through the existing extern with an options dictionary `{kAXTrustedCheckOptionPrompt: true}` for `request` (`objc2-core-foundation` / `core-foundation` dictionary); input monitoring through `extern "C" { fn IOHIDCheckAccess(request: u32) -> u32; fn IOHIDRequestAccess(request: u32) -> bool; }` with `#[link(name = "IOKit", kind = "framework")]`, `kIOHIDRequestTypeListenEvent = 1`, access `0 = granted, 1 = denied, 2 = unknown`; the dummy `CGEventTapCreate` probe is deleted; `settings_url` returns the deep links above; `responsible_app` keeps `ps -o comm= -p PPID` (or `NSRunningApplication` of the parent pid) when not bundled.
5. **Window adapter.** `adapters/primary/permissions/macos/mod.rs`: an `NSWindow` (titled, closable, not resizable, 460×360 pt, centred, floating level so it is not lost behind other windows) with an `NSStackView` of three rows, each an `NSStackView` of `NSImageView` (symbol), two `NSTextField` labels, a status `NSTextField` with a coloured `NSImageView` dot or a `NSButton` badge, and the action `NSButton`; a footer label and the `Quit` / `Continue` buttons; a target object (`define_class!`) with `request:` (the row's tag says which permission), `openSettings:`, `quit:`, `continue:`; a 2 s `NSTimer` while shown. Cargo features: `NSStackView`, `NSTimer` (foundation), `NSImageView`, `NSButton`, `NSWindow` (present), `NSWindowController` optional. Mapping functions (status → badge text and colour, permission → row texts, responsible app → button text) in `mapping.rs`, unit-tested.
6. **Runtime integration.** `Runtime::start` asks the gate before building the pipeline: all granted → proceed; else state `WaitingForPermissions(report)` and no pipeline. A `permissions` thread of the runtime re-checks every 5 s while Ready and every 2 s while waiting (or the window is open); on all granted → start the pipeline (reload path); on a revocation → stop the pipeline (cancel in progress with the notice through the indicator), state WaitingForPermissions. The window observes the runtime: it opens on WaitingForPermissions, refreshes rows on each report, enables `Continue` when all granted. The menu's `Permissions…` opens it in any state.
7. **Docs** as listed in What Changes; the agent-kit tech matrix PR.

Pitfalls: `IOHIDCheckAccess` for an unbundled binary reports the parent terminal's state — correct, that is who holds the permission; `AXIsProcessTrustedWithOptions` with prompt shows the dialog once per process and adds the app to the list unticked — the row still says Not granted until the user ticks it; the microphone request's completion block runs on an arbitrary thread — only set an atomic and let the poll pick it up; the window must be created on the main thread and shown with `makeKeyAndOrderFront` and `NSApp.activate` so it is in front of the app the user was in; do not call `activateIgnoringOtherApps` repeatedly (steals focus on every poll); a revoked Input Monitoring makes rdev stop delivering events silently — the 5 s poll is the only detection.

## Impact

- `app/src/core/application/{permission_probe.rs,permissions_gate.rs}` (new gate, `preflight.rs` deleted), `app/src/core/domain/permission.rs`
- `app/src/adapters/secondary/permissions/macos/mod.rs` (rewritten), new `app/src/adapters/primary/permissions/macos/{mod.rs,mapping.rs}`
- `app/src/runtime/` (gate before the pipeline, the permissions thread, the new state), `app/src/adapters/primary/menubar/macos/` (item, waiting state)
- `app/Cargo.toml` (`block2`, the `NSBundle` feature), `Cargo.lock`
- tests: unit in gate and mappings; new `app/tests/features/component/permissions_gate.feature` + `app/tests/steps/permissions_gate.rs`; `app/tests/permissions_macos_it.rs` (statuses are one of three values; URLs well formed; `responsible_app` names the parent when unbundled)
- `docs/architecture/context.md`, `docs/ubiquitous-language.md`, `docs/project-map.md`, `docs/architecture/overview.md`, `README.md`; agent-kit tech matrix (separate PR); `mockups/`
- `openspec/specs/menu-bar-presence`, `app-runtime` deltas
- Checks: `just test` after every edit; `just check` after the last group; `just full-check` before the merge; manual run by the user (permissions area): remove each permission in System Settings, launch, request and grant from the window, dictate; revoke one while running.

## Open points

- The window level: floating (proposed, so it is seen) or normal.
- Whether `Continue` is needed at all (the pipeline starts as soon as all are granted): proposed keep it as the explicit close; the window also closes by itself 2 s after the last grant (open point, the mockups decide).
- The poll interval while running: proposed 5 s (three cheap calls); alternative: a distributed notification for TCC changes (none public).
- Whether to drop the parent-process naming entirely once the app is always bundled: proposed keep it (developers run from a terminal daily).

## Release

SemVer: MINOR — a new capability and the end of exit code 2 for permissions; pre-1.0.
Target version: 0.30.0 (corrected at merge with `kit version-check`).
