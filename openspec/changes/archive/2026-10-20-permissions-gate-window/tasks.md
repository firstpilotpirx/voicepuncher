# Tasks

## 1. Gate use case and three-valued probe port

- [x] 1.1 Red: unit tests in `app/src/core/application/permissions_gate.rs` (mock probe) — report of three statuses, `missing()`, `all_granted()`, responsible app, one log line naming all three statuses; run, see them fail
- [x] 1.2 Green: `Permission` in `app/src/core/domain/permission.rs`, `PermissionStatus` beside it, the probe port (`status`, `request`, `settings_url`, `responsible_app`) in `app/src/core/application/permission_probe.rs`, `PermissionsGate` and `PermissionReport` in `permissions_gate.rs`; tests from 1.1 pass
- [x] 1.3 Delete `app/src/core/application/preflight.rs`; move its still-meaningful tests to the gate; `app/tests/architecture.rs` passes
- [x] 1.4 `docs/ubiquitous-language.md`: retire Preflight; add Permissions gate, Permission status, Permissions window
- [x] 1.5 `just test`; commit

## 2. macOS probe adapter

- [x] 2.1 Add `block2` pinned exactly in `app/Cargo.toml` with the features the probe needs (no `objc2-av-foundation`, design D3); `cargo build -p voicepuncher` passes
- [x] 2.2 Red: `app/tests/permissions_macos_it.rs` — every status is one of three values, `settings_url` for each permission is the exact deep link, `responsible_app` names the parent when unbundled; run, see them fail
- [x] 2.3 Green: rewrite `app/src/adapters/secondary/permissions/macos/mod.rs` — microphone through the AVFoundation status and request, accessibility through `AXIsProcessTrustedWithOptions` with and without the prompt option, input monitoring through `IOHIDCheckAccess` / `IOHIDRequestAccess`; the dummy event tap and the cpal stream probe deleted; tests from 2.2 pass
- [x] 2.4 `just test`; commit

## 3. Runtime integration

- [x] 3.1 Red: scenarios of `app-runtime` "The runtime waits for permissions" in `app/tests/features/component/app_runtime.feature` with steps in `app/tests/steps/app_runtime.rs` (mock probe driven by steps); run, see them fail
- [x] 3.2 Red: `app/tests/features/component/permissions_gate.feature` with steps in `app/tests/steps/permissions_gate.rs` — dictation starts only with all three, the last grant starts it, revocation while idle and during a recording (cancel sound, notice naming the permission), granting again resumes, not asked yet versus denied, file mode needs no permission, a change is logged; run, see them fail
- [x] 3.3 Green: the state Waiting for permissions with the missing list in the runtime, the gate asked before the pipeline starts, the permissions thread (2 s while waiting, 5 s while Ready) starting the pipeline on the last grant and stopping it on a revocation with the failure notice; tests from 3.1 and 3.2 pass
- [x] 3.4 `app/src/main.rs`: exit code 2 for a missing permission removed in hotkey mode; file mode skips the gate; existing `file_transcription.feature` still passes
- [x] 3.5 `docs/architecture/context.md`: the permission flow and sequence diagram (gate and window instead of Preflight → exit 2)
- [x] 3.6 `just test`; commit

## 4. Permissions window

- [x] 4.0 Mockups: `mockups/permissions.html` with `?state=` and `?bg=`, screenshots of every state of `mockups/design-spec.md` § 2 into `mockups/states/` with the built-in browser
- [x] 4.1 Red: unit tests in `app/src/adapters/primary/permissions/macos/mapping.rs` — status → badge text and colour, permission → name, why and symbol, status → Request… / Open System Settings / no button, responsible app → developer line, revoked permission → revoked line, Continue enabled only when all granted; run, see them fail
- [x] 4.2 Green: `mapping.rs` passes the tests from 4.1
- [x] 4.3 `app/src/adapters/primary/permissions/macos/mod.rs`: the AppKit window from `mockups/design-spec.md` (rows, badges, buttons, footer, developer and revoked lines), actions request, open settings through `NSWorkspace`, quit and continue, a 2 s refresh and a refresh when the app becomes active, floating level with focus taken only on first show; opens on Waiting for permissions; `cargo build -p voicepuncher` passes
- [x] 4.4 `app/tests/architecture.rs` covers the new primary adapter's dependency direction; passes
- [x] 4.5 `just test`; commit

## 5. Menu bar

- [x] 5.1 Red: unit tests in the menu bar mapping — Waiting for permissions gives the waiting icon and the status line "Waiting for permissions — open Permissions…"; the menu holds Permissions… under the status line in every state; run, see them fail
- [x] 5.2 Green: the menu bar adapter maps the new state and adds Permissions…, which brings the window to the front; tests from 5.1 pass
- [x] 5.3 `just test`; commit

## 6. Docs, version and checks

- [x] 6.1 `README.md` § macOS permissions (the window, the three panes, the terminal case), `docs/project-map.md`, `docs/architecture/overview.md` (the permission row)
- [x] 6.2 Bump `version` in `app/Cargo.toml` to 0.30.0 (corrected with `kit version-check`)
- [x] 6.3 `just test`; `just check`; commit
- [x] 6.4 Manual run by the user (README § Release checklist): remove each permission in System Settings, launch, request and grant each from the window, check each Open System Settings pane, dictate; revoke Accessibility while running and see the window reopen within 5 s; compare the window in light and dark with the mockups; paste a healthy log

## Workflow follow-up

- Agent-kit tech matrix PR, section "macOS Desktop": AVFoundation authorization status and request, AX prompt option, IOHID check and request access, System Settings deep links.
