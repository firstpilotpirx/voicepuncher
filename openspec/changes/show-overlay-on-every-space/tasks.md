## 1. The pill off the current desktop (D1, D3)

- [ ] 1.1 Red: scenario "Windows dropped from the current desktop are built again on show" under the rule "The overlay appears without rebuilding its windows" in `app/tests/features/component/visual_feedback.feature`, steps in `app/tests/steps/visual_feedback/overlay/windows.rs` with the seam `pretend_off_the_current_desktop`; `just test-one` fails (no rebuild); commit it red on its own
- [ ] 1.2 Green: `Redraw::draw` checks `isOnActiveSpace` of the shown pills and rebuilds and draws once more when one is off, with a warning (D1); `just test-one` green, `just static`
- [ ] 1.3 `just test`; `kit ship -m "fix(overlay): build the windows again when the pill is off the current desktop"`

## 2. A desktop switch (D2)

- [ ] 2.1 Red: scenario "A desktop switch during a recording brings the overlay along", driven through `desktop_switched`; `just test-one` fails
- [ ] 2.2 Green: the screen watcher observes `NSWorkspaceActiveSpaceDidChangeNotification` and calls the handler that runs the check of D1 while a state shows; `just test-one` green, `just static`
- [ ] 2.3 `just test`; `kit ship -m "fix(overlay): bring the overlay along on a desktop switch"`

## 3. No old windows left (D4)

- [ ] 3.1 Red: scenario "A rebuild leaves no old overlay windows" (the app's window count before and after a rebuild); `just test-one` fails
- [ ] 3.2 Green: dropping a `WindowSet` closes its windows; `just test-one` green, `just static`
- [ ] 3.3 `just test`; `kit ship -m "fix(overlay): close the old windows on a rebuild"`

## 4. Version

- [ ] 4.1 `kit version-check show-overlay-on-every-space`; bump `version` in `app/Cargo.toml` to 0.47.1 (or the version it names)
- [ ] 4.2 `just test`; `kit ship -m "chore(release): bump version to 0.47.1"`
