## 1. Checklist

- [x] 1.1 Write `docs/acceptance-run.md`: one item per checked requirement of the capabilities in design.md, grouped install → permissions → model download → menu bar → settings → launch at login → dictation → voice commands → quit and relaunch
- [x] 1.2 `just test` and commit

## 2. Preparation

- [x] 2.1 Get the disk image of the assembled `main` (release `.dmg` of the same version, else `just dmg` on `origin/main`)
- [x] 2.2 Stop the user's app and services; move `~/.voicepuncher/config.toml` and `history` aside, copy an installed `/Applications/voicepuncher.app` aside
- [x] 2.3 Request computer-use access for Finder, voicepuncher, System Settings and TextEdit
- [x] 2.4 Note the disk image's version and source in the report draft; commit

## 3. Run

- [x] 3.1 Install: open the disk image, drag the app to Applications, start it
- [x] 3.2 Permissions window and model download up to a working app (the user grants the permissions)
- [x] 3.3 Menu bar: every item and sub-item
- [x] 3.4 Settings window: every setting changed, applied and kept after a restart; launch at login on and off
- [x] 3.5 Dictation into TextEdit with synthesized speech; every voice command
- [x] 3.6 Quit and relaunch
- [x] 3.7 Record each item's result in the report draft; commit

## 4. Wrap-up

- [x] 4.1 Quit the installed app, undo launch at login, move the user's files and app back, restart the user's services
- [x] 4.2 Write the report `docs/acceptance-runs/<date>.md`; send failure screenshots to the user
- [x] 4.3 File every failure as one change with `/create-change`
- [x] 4.4 `just test` and commit

## 5. Version

- [x] 5.1 No version bump (no release); manual checks by the user: granting the three permissions during 3.2
