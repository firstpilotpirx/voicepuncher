## Context

See proposal.md § Why. The Permissions window (`app/src/adapters/primary/permissions/macos/`) already re-draws from the gate every 2 s; its pure part is `mapping.rs` (`WindowModel`, `badge`, `button`, `why`). The login switch's pure part is `settings/macos/login.rs` over `LoginItem`, whose macOS adapter maps `SMAppServiceStatus` in `status_of`. The menu's pure part is `menubar/macos/mapping.rs` (`StatusItemState::icon`, `status_line`); `RuntimeState::WaitingForPermissions { missing, .. }` already names the missing permissions. The settings window builds its notes under fields once per opening (`window.rs` `build`).

The decisions recorded with the user in `orchestration.yaml` (`prepared`) are taken as they are; this design only places them in the code.

## Goals / Non-Goals

**Goals:** every decision below lives in a pure function with unit tests; the AppKit code only draws what those functions return.

**Non-Goals:** no change to how the gate decides a permission is granted (`AXIsProcessTrustedWithOptions`, `IOHIDCheckAccess`); a stale TCC entry is explained, not repaired. No live update of an already open settings window.

## Decisions

1. **The row's line.** `mapping::row_line(permission, shown, opened, macos_major) -> String` replaces the fixed `why` line of a row: a granted or checking row keeps `why`; a row not granted says `Switch voicepuncher on in <pane>` with `pane_name(permission, macos_major)` ("Device Control and Data Access" for Accessibility from macOS 27, "Accessibility" before; "Microphone", "Input Monitoring"); once the user opened System Settings from that row (`opened`) the line becomes the hint — for Input Monitoring `Not listed? Add voicepuncher with +.`, for the others `Already on? Remove voicepuncher with − and add it again with +.`. The window keeps the permissions opened in its state beside `pending`. The macOS major version comes from `NSProcessInfo.operatingSystemVersion` in the window. Alternative: an extra hint line under the status badge — rejected, the row has a fixed height and the reason line already wraps to two lines.
2. **Polling every second.** `REFRESH_MS` becomes 1000 (decision "Poll every second").
3. **Input Monitoring after a denial.** `mapping::settings_steps(permission, from_bundle) -> Vec<SettingsStep>` with `SettingsStep::{Request, OpenPane, RevealBundle}`: for Input Monitoring `[Request, OpenPane]` plus `RevealBundle` from a bundle; for the others `[OpenPane]`. The window runs the steps: `Request` through the gate on the request thread (`IOHIDRequestAccess` lists a denied app where macOS allows it), `OpenPane` the URL as today, `RevealBundle` `NSWorkspace.activateFileViewerSelectingURLs` with `NSBundle.mainBundle.bundleURL`.
4. **Login item never registered.** `status_of` maps `SMAppServiceStatus::NotFound` to `NotRegistered`: the adapter returns Unavailable only outside a bundle (`installed` is false), which it checks before asking the system; a refused registration already shows the system's reason (requirement "A failure is told"). The dimmed look: `settings/hints.rs` `switch_alpha(enabled)` is 1.0 or 0.4 and `LoginRow::show` sets it as the switch's alpha value.
5. **The menu.** `IconState::MissingPermission` (SF Symbol `lock.shield`, template) for `WaitingForPermissions`; `standby_line` reads `Waiting for <names>…` with the names joined by `settings/hints.rs` `joined_names` ("A", "A and B", "A, B and C"). The status line item gets the Permissions action, enabled only while `StatusItemState::status_opens_permissions()` is true (waiting for permissions and no dictation).
6. **Settings.** `PermissionsWait` (`app/src/runtime/permissions/wait.rs`) is a `RuntimeObserver` that keeps the last `missing` list in an `Arc<Mutex<Vec<Permission>>>`; `menubar_app.rs` hands `SettingsWindow::with_permissions_wait` a closure reading it (the settings adapter never imports the runtime), and `build` places `settings/hints.rs` `waiting_note(&missing)` under the voice control switch through `placed_notes` when it is `Some`: `Waiting for <names> — dictation and voice control start once it is granted`. Alternative: push changes into an open window — rejected as a non-goal; the note is current at every opening, as the spec asks.

### Test mapping

| Spec scenario | Test |
|---|---|
| permissions-gate: The Accessibility pane is named as macOS 27 names it | unit: `permissions/macos/mapping.rs` `should_name_the_accessibility_pane_by_the_macos_version` |
| permissions-gate: The Accessibility pane keeps its old name before macOS 27 | unit: `mapping.rs` `should_name_the_accessibility_pane_by_the_macos_version` |
| permissions-gate: Open System Settings lands on the permission's page | integration: `app/tests/it/permissions_macos.rs` `should_link_every_permission_to_its_privacy_page`, unchanged |
| permissions-gate: Request shows the system prompt | unit: existing `mapping.rs` `should_show_checking_while_a_request_is_not_answered`, unchanged |
| permissions-gate: A granted row has no button | unit: existing `mapping.rs` `should_offer_request_for_not_asked_yet_settings_for_denied_and_nothing_for_granted`, unchanged |
| permissions-gate: Continue waits for every permission | unit: existing `mapping.rs` `should_enable_continue_only_when_every_permission_is_granted`, unchanged |
| permissions-gate: Continue closes the window once all are granted | unit: the same existing test; closing is unchanged AppKit code |
| permissions-gate: A grant in System Settings shows up in the open window | `mapping.rs` `should_refresh_every_second` (`REFRESH_MS == 1000`) |
| permissions-gate: Accessibility switched on for an earlier build / No hint before System Settings was opened | `mapping.rs` `should_hint_to_add_the_app_again_only_after_system_settings_opened` |
| permissions-gate: Input Monitoring denied | `mapping.rs` `should_request_open_and_reveal_for_input_monitoring`, `should_hint_to_add_input_monitoring_with_plus` |
| launch-at-login: Run from a build folder | `settings/hints.rs` `should_dim_a_disabled_switch`, existing unavailable test |
| launch-at-login: A bundle that never registered can switch / Switching on a bundle that never registered | `loginitem/macos/tests.rs` `should_take_not_found_as_not_registered`; component: `launch_at_login.feature` "An unregistered app shows the switch off" (off, enabled, no note) and "Switching on registers the app" |
| menu-bar-presence: A missing permission at launch / Two missing permissions | `menubar/macos/mapping/permission_wait_tests.rs` `should_name_the_missing_permissions_with_their_own_icon` |
| menu-bar-presence: The waiting item opens the Permissions window | `mapping/permission_wait_tests.rs` `should_let_the_status_line_open_permissions_only_while_waiting_for_them` |
| settings-window: Voice control on while Accessibility is missing / No line once every permission is granted | `settings/hints.rs` `should_note_the_missing_permissions_under_voice_control`, `should_place_the_waiting_note_under_the_voice_control_switch`; `runtime/permissions/wait.rs` `should_keep_the_missing_permissions_of_the_last_state` |

## Risks / Trade-offs

- [`IOHIDRequestAccess` may not list a denied app on macOS 27] → the bundle is shown in Finder and the row says to add it with +.
- [The pane URL anchor for Accessibility may differ on macOS 27] → the anchor stays `Privacy_Accessibility`, which System Settings still resolves to the renamed pane; the row names the pane, so the user finds it even if it lands on Privacy & Security.
- [No manual run tonight] → the morning report lists the checks of README § Release checklist (permissions, bundle) to do by hand (decision recorded with the user).
