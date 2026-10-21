No architectural decision in the app: the change adds a checklist document and one run's report, no code.

## Decisions

### The checklist is a repo document derived from the specs

`docs/acceptance-run.md` keeps the checklist for future runs. Each item names the spec requirement it checks (capabilities `app-distribution`, `permissions-gate`, `local-services` model download, `menu-bar-presence`, `settings-window`, `launch-at-login`, `push-to-talk`, `text-insertion`, `voice-control`), so a later change to a spec shows which item to update. Items are grouped in the order a new user meets them: install, permissions, model download, menu bar, settings, launch at login, dictation, voice commands, quit and relaunch.

### What is installed

The disk image built from the assembled `main` after every dependency landed: the GitHub Release's `.dmg` when its version equals the version on `main`, otherwise `just dmg` on a clean checkout of `origin/main` (signed with the user's identity, as a release is). The run never publishes a release.

### A clean first run in the user's own account

A fresh macOS user account needs the user's admin password and an account creation Claude may not perform, so the run uses the user's account and makes it look new instead: it stops the user's running app and services (`just stop-model-servers`), moves `~/.voicepuncher` aside to `~/.voicepuncher.acceptance-backup` (moved, never deleted) and removes an installed `/Applications/voicepuncher.app` only after copying it aside the same way. At the end it quits the installed app, unregisters launch at login if the run switched it on, and moves everything back. Models are downloaded again by the installer, which is what the model download items check.

### Who does what

Claude drives Finder, the app's windows and menu, System Settings (read only) and TextEdit through computer use, asking for access to each app once. Granting the three permissions in System Settings and any password prompt are the user's: Claude asks the user to do them and waits, and records the item as blocked when no one answers. Speech reaches the microphone from the speakers (`say`); when recognition fails on a phrase that synthesized speech passes in E2E, the item is retried once at a higher volume and then recorded as failed with the heard text, and a loopback device is noted as the follow-up.

### Report and bugs

The report `docs/acceptance-runs/<date>.md` copies the checklist with a result per item (pass, fail, blocked) and a one-line observation for each failure; failure screenshots are sent to the user, not committed. All failures of the run are filed as one change with `/create-change` (one change per bug hunt).

## Dependencies

The run checks the assembled product, so it waits for retire-cli-setup-wizard (the visual installer), launch-at-login, bundled-engines, model-download (the installer's model download it reworks) and service-reliability-review (the last other change of the backlog).

## Test levels

The change adds no spec scenario; the run itself is a manual run of existing scenarios. `just test` guards the repository after the document commits.
