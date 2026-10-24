## 1. Permissions window

- [ ] 1.1 Red: `mapping.rs` tests `should_name_the_accessibility_pane_by_the_macos_version`, `should_hint_to_add_the_app_again_only_after_system_settings_opened`, `should_hint_to_add_input_monitoring_with_plus`, `should_request_open_and_reveal_for_input_monitoring`, `should_refresh_every_second`; commit them red (`test: reproduce the permissions window bugs`)
- [ ] 1.2 Green: `pane_name`, `row_line`, `settings_steps`, `REFRESH_MS = 1000`; the window keeps the opened permissions, draws `row_line` and runs the steps
- [ ] 1.3 `just test`; commit

## 2. Start at login

- [ ] 2.1 Red: `loginitem/macos/tests.rs` `should_take_not_found_as_not_registered`, `login.rs` `should_dim_a_disabled_switch`; commit them red (`test: reproduce the login switch bugs`)
- [ ] 2.2 Green: `status_of` maps `NotFound` to `NotRegistered`; `switch_alpha`, set by `LoginRow::show`
- [ ] 2.3 `just test`; commit

## 3. The wait named in the menu and in Settings

- [ ] 3.1 Red: `mapping/tests.rs` `should_name_the_missing_permissions_with_their_own_icon`, `should_let_the_status_line_open_permissions_only_while_waiting_for_them`; `login.rs` `should_note_the_missing_permissions_under_voice_control`; `runtime/permissions_wait.rs` `should_keep_the_missing_permissions_of_the_last_state`
- [ ] 3.2 Green: `IconState::MissingPermission`, `names`, the waiting status line, `status_opens_permissions` and the status line's action; `PermissionsWait`, `waiting_note`, `SettingsWindow::with_permissions_wait`, wired in `menubar_app.rs`
- [ ] 3.3 README § First launch and § Release checklist name both pane names; `docs/ubiquitous-language.md` gains "missing-permission icon"
- [ ] 3.4 `just test`; commit

## 4. Release

- [ ] 4.1 Bump `version` in `app/Cargo.toml` to the next PATCH above `main` (0.38.3); the proposal's target version follows
- [ ] 4.2 `just test`; commit
