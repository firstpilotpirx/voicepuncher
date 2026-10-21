## 1. Login item port and macOS adapter

- [ ] 1.1 Red: unit tests for the installed-bundle detection from an executable path (`…/X.app/Contents/MacOS/X` installed, a `target/` path not) and the mapping of `SMAppServiceStatus` values to `LoginItemStatus` (3 and unknown → Unavailable)
- [ ] 1.2 Green: `core/application/login_item.rs` (`LoginItem`, `LoginItemStatus`), `adapters/secondary/loginitem/macos/mod.rs` over `objc2-service-management = "=0.3.2"`, returning Unavailable without calling the framework outside a bundle
- [ ] 1.3 Red: `app/tests/loginitem_macos_it.rs` — `status` on the test binary returns Unavailable and `register` refuses without touching the system; Green as needed
- [ ] 1.4 `just test` and commit

## 2. The login switch in the General tab

- [ ] 2.1 Red: `app/tests/features/component/launch_at_login.feature` rules "The switch shows the system's state", "Switching on registers, switching off unregisters", "Unavailable outside an installed app", "A failure is told", with steps in `app/tests/steps/launch_at_login.rs` over a mock `LoginItem`
- [ ] 2.2 Green: `settings/macos/login.rs` `LoginSwitch` (shown state, note, approval button; toggle with rollback and the reason)
- [ ] 2.3 The General tab's login row (switch, note, `Open Login Items Settings…` opening `x-apple.systempreferences:com.apple.LoginItems-Settings.extension`), its target actions, the refresh when the window becomes key, the failure on the status line; wired in `main.rs`
- [ ] 2.4 `just test` and commit

## 3. The one-time offer and the app's record

- [ ] 3.1 Red: field-table unit test — `app.launch_at_login_asked` is a kept key without a control; config unit test — `[app]` defaults to not asked
- [ ] 3.2 Green: `[app] launch_at_login_asked` in `config.rs`, `KEPT` in the field table
- [ ] 3.3 Red: the rule "The offer is made once" in `launch_at_login.feature` (asks once, Start at Login registers, Not Now remembered, no second ask after a reload or a relaunch, nothing for an app that is not installed)
- [ ] 3.4 Green: `LoginOffer` (whether to ask; the answer registers and records it with `ConfigDocument`) and its runtime observer showing the `NSAlert` on the main thread at the first Ready; wired in `main.rs`
- [ ] 3.5 `just test` and commit

## 4. Launched at login

- [ ] 4.1 Red: unit test of the launch origin from the open-application Apple event (`'lgit'` → at login; anything else or no event → nothing); the rule "A launch at login is logged" in `launch_at_login.feature`
- [ ] 4.2 Green: the origin function and the log line, read at `NSApplicationDidFinishLaunchingNotification` through `NSAppleEventManager`
- [ ] 4.3 `just test` and commit

## 5. Docs and release

- [ ] 5.1 README § First launch (the offer, where to switch it off) and § Settings (the switch, no config key); `docs/ubiquitous-language.md` (Login item); `docs/architecture/overview.md` § Technology Choices (login item row); `app/config.example.toml` (`[app]`)
- [ ] 5.2 Bump `version` in `app/Cargo.toml` to the next MINOR above `main` (`kit version-check launch-at-login`)
- [ ] 5.3 `just check` once and commit
