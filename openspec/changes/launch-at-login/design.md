# Design

## Context

`SMAppService` (ServiceManagement, macOS 13+) registers the main app as a login item; the user sees and controls it in System Settings → General → Login Items; `objc2-service-management` 0.3.2 binds it. The app is a bundle since `app-bundle-and-dmg` and has a Settings window since `settings-window`; the runtime's Ready state since `restartable-app-runtime`.

## Decisions

**D1 — A port with a mock, the framework behind an adapter.** `LoginItem` in the application layer keeps `objc2-service-management` out of the window code and lets the component tests cover the five requirements without touching the user's login items.

**D2 — The system is the source of truth; the config remembers only the question.** The switch reads `status()` every time the tab opens and when the window becomes key; `config.toml` holds only `launch_at_login_asked`, never the registration itself (two sources would drift when the user changes it in System Settings).

**D3 — An "act at once" control kind in the field model.** The switch is excluded from Save, Revert and the dirty flag; the field model gets `Action` for it, so the settings window keeps one builder.

**D4 — Unavailable outside an installed bundle, decided by the executable path.** `cargo run` and tests never call the framework; the switch says why it is disabled.

**D5 — One offer after the first Ready.** Asking before the onboarding would be noise; asking after the first successful start is when the user knows the app works.

## Risks / Trade-offs

- [macOS refuses an unsigned or relocated bundle] → the error text is shown; the self-signed signature of `app-bundle-and-dmg` is enough for `SMAppService`.
- [A test registers the test binary] → forbidden by the pitfalls; the integration test is status-only.
- [`RequiresApproval` confuses the user] → the note and the button to the Login Items pane.

## Test levels

| Scenario | Level |
|---|---|
| Status mapping; installed-bundle detection from a path | unit tests in the adapter and the wiring |
| The switch shows the state; on registers, off unregisters; unavailable; failure told; the offer once | component `launch_at_login.feature` (mock `LoginItem`, field model, runtime with mocks) |
| `status` on an unbundled binary returns Unavailable without panicking | adapter integration `loginitem_macos_it.rs` |
| Login Items lists the app; a logout and login starts it | manual run by the user |
