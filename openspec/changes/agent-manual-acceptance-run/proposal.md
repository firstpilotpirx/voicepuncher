## Why

The app is moving from a console tool to a visual app: app bundle and DMG, permissions gate window, menu bar (tray) presence, settings window, model download, launch at login, bundled engines, and the console setup wizard is retired. Each change is checked by its own tests and a manual run by the user, but nobody checks the whole installed product end to end, the way a new user meets it. The manual run today is done by the user by hand. Claude can drive the mouse and keyboard (computer use) once the user grants access, so the final acceptance run can be done by Claude itself.

## What Changes

- Claude runs a full manual acceptance pass of the released app through computer use, asking the user for access to each app it needs (Finder, the app, System Settings, a text editor) and waiting for the grant.
- Install from scratch: open the DMG, drag the app to Applications, start it, walk through the visual installer (permissions gate window, model download) up to a working app.
- Tray: open the menu bar menu and exercise every item and sub-item; check each does what its label says.
- Settings window: open it, change each setting, check the change takes effect and survives a restart (including launch at login).
- Speech: dictate into a text field (synthesized speech played to the microphone, e.g. macOS `say`) and check the pasted text; speak every voice command and check its effect.
- Report: a checklist of every item with pass/fail, screenshots of failures, and each bug found filed with `/create-change` (one change for all bugs of this run).

Current stage: PoC. Target stage: PoC.
Scope: one end-to-end acceptance run of the installed app, driven by Claude through computer use, after every installer and GUI change of the backlog is merged.
Acceptance bar: every checklist item ran and has a recorded result; every failure is filed as a change; the report is shown to the user.
Out of scope: fixing the bugs found (they are filed, not fixed here); automating this run as a permanent test.

## Capabilities

### Modified Capabilities

- None: the run checks existing behavior and adds no requirement.

## Impact

- No code change expected; possibly a checklist document (e.g. `docs/acceptance-run.md`) and the run's report.
- Uses the release DMG from `just release`, computer-use tools, macOS TCC permission prompts granted by the user.

## Open points

- Whether to keep the checklist as a repo document for future runs — proposed: yes, `docs/acceptance-run.md`.
- How speech reaches the microphone: playing `say` through speakers into the mic vs. a loopback audio device — proposed: speakers first, loopback device if recognition is unreliable.
- TCC permission prompts (Microphone, Accessibility, Input Monitoring) may need the user's password or a click Claude is not allowed to make — proposed: Claude stops and asks the user to grant them.
- Install on the user's main system vs. a fresh macOS user account — proposed: a fresh user account, so existing settings and models do not hide first-run problems.

## Release

SemVer: no release — the run changes no behavior.
Target version: 0.25.0.
