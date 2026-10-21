# Design

## Context

`SMAppService` (ServiceManagement, macOS 13+) registers the main app as a login item; the user sees and controls it in System Settings → General → Login Items; `objc2-service-management` 0.3.2 binds it. The app is a bundle since `app-bundle-and-dmg` and has a Settings window since `settings-window`; the runtime's Ready state since `restartable-app-runtime`.

## Decisions

**D1 — A port with a mock, the framework behind an adapter.** `LoginItem` in the application layer keeps `objc2-service-management` out of the window code and lets the component tests cover the five requirements without touching the user's login items.

**D2 — The system is the source of truth; the config remembers only the question.** The switch reads `status()` every time the tab opens and when the window becomes key; `config.toml` holds only `launch_at_login_asked`, never the registration itself (two sources would drift when the user changes it in System Settings).

**D3 — The switch acts at once.** The switch is excluded from Save, Revert and the dirty flag; how it stays out of them is D7.

**D4 — Unavailable outside an installed bundle, decided by the executable path.** `cargo run` and tests never call the framework; the switch says why it is disabled.

**D5 — One offer after the first Ready.** Asking before the onboarding would be noise; asking after the first successful start is when the user knows the app works. The offer is a `RuntimeObserver` that fires at most once per run (a reload reaches Ready again), and only when the config does not record the question and the login item is `NotRegistered` (an app already registered, or unavailable, is never asked).

**D6 — Pure models under the AppKit glue.** `LoginSwitch` (what the switch shows for a status; what a toggle does, with the failure rolled back) and `LoginOffer` (whether to ask; what an answer does to the login item and the config document) hold every decision and take the `LoginItem` port and a config path; the window row and the `NSAlert` only show them. The component scenarios drive these models with a mock `LoginItem` and a real temporary `config.toml`, as `settings_window.feature` drives `SettingsSession`.

**D7 — Not a `Field`.** The switch is not a configuration option, so it is not a row of the field table: the General tab's layout adds a login row after its fields, with its own target action (`loginToggled:`, `openLoginItems:`) and a refresh on `windowDidBecomeKey:`. No `Action` control kind is needed: a control outside the field table already stays out of Save, Revert and the dirty flag.

**D8 — The app's own records.** `[app] launch_at_login_asked` is listed in the field table's `KEPT` list (beside `RETIRED`): a key the app writes itself and shows no control for; the "every key has a field" test excludes both lists.

**D9 — Launched at login, from the open-application Apple event.** At `NSApplicationDidFinishLaunchingNotification` the current Apple event is the `kAEOpenApplication` event; its `keyAEPropData` parameter is `keyAELaunchedAsLogInItem` (`'lgit'`) when the login item started the app. A pure function maps (event id, property) to the launch origin and the log line; the glue reads the event through `NSAppleEventManager`. The line is informational: a missing event logs nothing.

## Risks / Trade-offs

- [macOS refuses an unsigned or relocated bundle] → the error text is shown; the self-signed signature of `app-bundle-and-dmg` is enough for `SMAppService`.
- [A test registers the test binary] → forbidden by the pitfalls; the integration test is status-only.
- [`RequiresApproval` confuses the user] → the note and the button to the Login Items pane.

## Test levels

| Scenario | Level |
|---|---|
| Status mapping; installed-bundle detection from a path; launch origin from the Apple event | unit tests in the adapter |
| A registered app shows the switch on | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| An unregistered app shows the switch off | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| A registration waiting for approval | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| A change made in System Settings is shown | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| Switching on registers the app | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| Switching off unregisters the app | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| Run from a build folder | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| A refused registration | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| The first Ready asks and Start at Login registers | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| Not Now registers nothing and is remembered | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| The question is not asked again | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| Ready again after a reload asks nothing | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| An app that is not installed asks nothing | component `launch_at_login.feature` (mock `LoginItem`, `LoginSwitch`, `LoginOffer` over a temporary `config.toml`) |
| Started by the login item | component `launch_at_login.feature` (the launch log line for an open-application event from the login item) |
| A value from the file is shown | component `settings_window.feature` (unchanged, from `settings-window`) |
| An option the file does not set shows its default | component `settings_window.feature` (unchanged, from `settings-window`) |
| The microphone choice lists the Mac's input devices | component `settings_window.feature` (unchanged, from `settings-window`) |
| No option is left without a control | component `settings_window.feature` and a unit test of the field table (`table/tests.rs`), both without `RETIRED` and `KEPT` |
| The app's own record has no control | component `settings_window.feature` and a unit test of the field table (`table/tests.rs`) |
| Login Items lists the app; a logout and login starts it | manual run by the user |
