## Why

A push-to-talk tool is useful only while it runs; a user who installs voicepuncher from the DMG expects it to be there after every login, like any menu bar utility, without opening it by hand. Today nothing starts the app at login; a developer runs it from a terminal. macOS gives applications a supported way to register themselves as a login item (`SMAppService`, macOS 13 and later; the app requires 15.0), which shows the app under System Settings → General → Login Items, lets the user switch it off there, and needs no LaunchAgent file written by hand.

## What Changes

- The General tab of the Settings window (`settings-window`) gets a switch `Start voicepuncher when you log in`. It shows the system's truth: on opening the tab the app asks `SMAppService` for its status and sets the switch (`Enabled` → on; `NotRegistered` → off; `RequiresApproval` → on, with the note `Waiting for your approval in System Settings` and a button `Open Login Items Settings…` that opens `x-apple.systempreferences:com.apple.LoginItems-Settings.extension`; `NotFound` or any error → the switch disabled with the note `Available when voicepuncher runs as an installed app` — the case of `cargo run` and of a bundle outside Applications that macOS does not accept). Switching it on calls `register`, off calls `unregister`; a failure shows its message in the status line and the switch goes back. The switch takes effect at once; it is not part of Save (it is not in `config.toml`).
- One-time offer: at the first launch of the installed app that reaches the Ready state (permissions granted, models present, services ready — after the onboarding, never before it), an `NSAlert` asks `Start voicepuncher when you log in?` with `Start at Login` and `Not Now`; the answer is remembered in a new config key `[app] launch_at_login_asked = true` (a new `[app]` section for application-level settings), so the question is asked once; `Not Now` leaves the switch for later. The alert is skipped when the app is not an installed bundle.
- When the app is launched at login it starts as always: the menu bar icon, the permissions gate only when something is missing, no window otherwise; the log says `launched at login` when the launch came from the login item (`NSAppleEventManager` current event is `kAEOpenApplication` with `keyAEPropLaunchedAsLogInItem`, or simply the parent process being `launchd` with no terminal — the implementation picks what is reliable and logs it as informational).
- Docs: README § First launch (the offer; where to switch it off; System Settings → General → Login Items), § Settings table (the switch, no config key); `docs/ubiquitous-language.md` (Login item); `docs/architecture/overview.md` § Technology Choices (login item row). Tech matrix (agent-kit PR, section "macOS Desktop"): launch at login — `SMAppService` through `objc2-service-management`, pinned `=0.3.2`; no hand-written LaunchAgent plist (it would not show in Login Items and would break with the bundle's identity).

Current stage: PoC. Target stage: Prototype (the installed app behaves like a resident menu bar utility).
Scope: the port, the macOS adapter, the switch, the one-time offer, the `[app]` config section, docs, tech matrix.
Acceptance bar: on the installed app the switch registers the login item (System Settings → General → Login Items lists voicepuncher), a logout and login starts the app with its menu bar icon; switching it off removes it from the list; the offer appears once and never again; `cargo run` shows the switch disabled with the note; the user confirms on their Mac.
Out of scope: a LaunchAgent or daemon; starting the services at login without the app; hiding the app at launch beyond what `LSUIElement` already does; a "start minimised" option (there is no window to minimise).

## Capabilities

### New Capabilities

- `launch-at-login`: the installed app can register itself as a login item from the Settings window, reflects the system's state, needs the user's approval when macOS asks for it, and offers it once after the first successful start.

### Modified Capabilities

- `settings-window`: the General tab's switch that acts at once and is not part of Save; the `[app]` section.

## Requirements to specify

1. The switch shows the system's state: opening the General tab shows whether the app is registered as a login item, and a registration that waits for the user's approval is shown with the way to System Settings.
2. Switching on registers, switching off unregisters: the change applies at once and the Login Items list of System Settings reflects it.
3. Unavailable outside an installed bundle: when the app does not run as an installed bundle the switch is disabled and says so.
4. The offer is made once: the first launch that reaches Ready as an installed bundle asks whether to start at login; the answer is remembered and the question never returns.
5. A failure is told: a registration that fails shows its reason and leaves the switch in the previous position.

## Implementation guide

1. **Port.** `core/application/login_item.rs`: `trait LoginItem { fn status(&self) -> LoginItemStatus; fn register(&self) -> Result<(), String>; fn unregister(&self) -> Result<(), String>; }` with `enum LoginItemStatus { NotRegistered, Enabled, RequiresApproval, Unavailable }`.
2. **Adapter.** `adapters/secondary/loginitem/macos/mod.rs` with `objc2-service-management = "=0.3.2"` (features `SMAppService`): `SMAppService::mainAppService()`, `status()` mapped from `SMAppServiceStatus` (`NotRegistered` 0, `Enabled` 1, `RequiresApproval` 2, `NotFound` 3 → `Unavailable`), `registerAndReturnError`/`unregisterAndReturnError` with the `NSError` text in the `Err`. Detect "installed bundle" as: `current_exe()` ends with `.app/Contents/MacOS/NAME` (any location; macOS decides acceptance — a bundle outside `/Applications` may still register); otherwise `Unavailable` without calling the framework.
3. **Settings tab.** In `settings/macos/tabs/general.rs`: the switch bound to the port, not to a config field (the field model gets a `Action` control kind for controls that act at once and are excluded from Save, Revert and the dirty flag); the note label and the `Open Login Items Settings…` button shown only for `RequiresApproval`; a refresh of the status when the window becomes key (the user may have changed it in System Settings).
4. **Offer.** In the runtime wiring: on the first transition to Ready, when `[app] launch_at_login_asked` is false and the app is an installed bundle, show the alert on the main thread; `Start at Login` → `register`; either answer → `ConfigDocument::set("app", "launch_at_login_asked", true)` and save (the `toml_edit` document of `settings-window`).
5. **Config.** `config.rs`: `[app]` with `launch_at_login_asked: bool` default false; `fields.rs` lists it as a hidden field (no control; the test of "every key has a field" accepts `Hidden`).
6. **Docs** as listed; the agent-kit tech matrix PR.

Pitfalls: never call `register` from a test or from `cargo test` binaries — it would add the test binary to the user's login items; the integration test calls `status` only; `SMAppService` returns `NotFound` for an unbundled binary and `registerAndReturnError` fails with a `kSMErrorInvalidSignature`-like error for an unsigned bundle — map both to a message, never panic; the deep link for Login Items is `x-apple.systempreferences:com.apple.LoginItems-Settings.extension` (not the Privacy pane); `RequiresApproval` appears when macOS shows the "Background Items Added" notification — the switch shows on with the note, not off; the alert must come after the onboarding windows closed, never over the permissions gate; the main app service needs `NSApplication` running (the adapter is called on the main thread from the window; the offer dispatches to the main thread).

## Impact

- new `app/src/core/application/login_item.rs`, `app/src/adapters/secondary/loginitem/macos/mod.rs`, `app/src/adapters/secondary/mod.rs`
- `app/src/adapters/primary/settings/macos/{fields.rs,tabs/general.rs}`, `app/src/runtime/` (the offer), `app/src/config.rs` (`[app]`)
- `app/Cargo.toml` (`objc2-service-management`), `Cargo.lock`
- tests: unit for the status mapping and the bundle detection; new `app/tests/loginitem_macos_it.rs` (`status` on the test binary returns `Unavailable` and never panics); component `app/tests/features/component/launch_at_login.feature` + `app/tests/steps/launch_at_login.rs` with a mock `LoginItem` (requirements 1–5: state shown, register/unregister, unavailable, the offer once, failure told)
- `README.md`, `docs/ubiquitous-language.md`, `docs/architecture/overview.md`; agent-kit tech matrix (separate PR)
- `openspec/specs/settings-window` delta
- Checks: `just test` after every edit; `just check` after the last group; `just full-check` before the merge; manual run by the user on the installed app: switch on, check Login Items, log out and in, switch off; the log pasted.

## Open points

- The offer after the first Ready (proposed) versus never asking and leaving the switch only: proposed to ask once, as most menu bar utilities do.
- `[app]` as the new section name (proposed) versus `[general]` (the tab's name): proposed `[app]` to keep the tab name free of config meaning.
- Whether a bundle outside `/Applications` should be allowed to register: proposed to let macOS decide and show its error.

## Release

SemVer: MINOR — a new capability; pre-1.0.
Target version: 0.35.0 (corrected at merge with `kit version-check`).
