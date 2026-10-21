## 1. Checklist

- [ ] 1.1 Write `docs/acceptance-run.md`: one item per checked requirement of the capabilities in design.md, grouped install → permissions → model download → menu bar → settings → launch at login → dictation → voice commands → quit and relaunch
- [ ] 1.2 `just test` and commit

## 2. Preparation

- [ ] 2.1 Get the disk image of the assembled `main` (release `.dmg` of the same version, else `just dmg` on `origin/main`)
- [ ] 2.2 Stop the user's app and services; move `~/.voicepuncher` and an installed `/Applications/voicepuncher.app` aside
- [ ] 2.3 Request computer-use access for Finder, voicepuncher, System Settings and TextEdit
- [ ] 2.4 Note the disk image's version and source in the report draft; commit

## 3. Run

- [ ] 3.1 Install: open the disk image, drag the app to Applications, start it
- [ ] 3.2 Permissions window and model download up to a working app (the user grants the permissions)
- [ ] 3.3 Menu bar: every item and sub-item
- [ ] 3.4 Settings window: every setting changed, applied and kept after a restart; launch at login on and off
- [ ] 3.5 Dictation into TextEdit with synthesized speech; every voice command
- [ ] 3.6 Quit and relaunch
- [ ] 3.7 Record each item's result in the report draft; commit

## 4. Wrap-up

- [ ] 4.1 Quit the installed app, undo launch at login, move the user's files and app back, restart the user's services
- [ ] 4.2 Write the report `docs/acceptance-runs/<date>.md`; send failure screenshots to the user
- [ ] 4.3 File every failure as one change with `/create-change`
- [ ] 4.4 `just test` and commit

## 5. Version

- [ ] 5.1 No version bump (no release); manual checks by the user: granting the three permissions during 3.2
