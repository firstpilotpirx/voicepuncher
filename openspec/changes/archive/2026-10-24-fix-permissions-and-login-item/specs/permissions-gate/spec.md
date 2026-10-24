## MODIFIED Requirements

### Requirement: Each permission is shown with its status and its remedy
The Permissions window SHALL show one row per permission with its name, one line of why the app needs it, its status — Granted, Not granted, Not asked yet, or Checking… while a request is pending — and one button: Request… for a permission not asked yet, Open System Settings for one not granted, none for one granted. A row not granted SHALL say in which System Settings pane to switch voicepuncher on, named as the running macOS names it — "Device Control and Data Access" for Accessibility on macOS 27 and later, "Accessibility" before — and Open System Settings SHALL open that pane.

#### Scenario: Open System Settings lands on the permission's page
- **GIVEN** Microphone is not granted
- **WHEN** the user presses Open System Settings in the Microphone row
- **THEN** System Settings opens on the Microphone page of Privacy & Security

#### Scenario: Request shows the system prompt
- **GIVEN** Input Monitoring was never asked for
- **WHEN** the user presses Request… in the Input Monitoring row
- **THEN** the system asks the user for Input Monitoring
- **AND** the row shows Checking… until the status is known

#### Scenario: A granted row has no button
- **GIVEN** Microphone is granted
- **WHEN** the Permissions window shows
- **THEN** the Microphone row shows Granted and no button

#### Scenario: The Accessibility pane is named as macOS 27 names it
- **GIVEN** the app runs on macOS 27 and Accessibility is not granted
- **WHEN** the Permissions window shows the Accessibility row
- **THEN** the row names the pane "Device Control and Data Access"

#### Scenario: The Accessibility pane keeps its old name before macOS 27
- **GIVEN** the app runs on macOS 26 and Accessibility is not granted
- **WHEN** the Permissions window shows the Accessibility row
- **THEN** the row names the pane "Accessibility"

### Requirement: The window is current and can be opened at any time
The Permissions window SHALL re-check every permission every second while it is open and when the app becomes active, and update its rows. Continue SHALL be enabled only when all three are granted and SHALL close the window; Quit SHALL end the app.

#### Scenario: A grant in System Settings shows up in the open window
- **GIVEN** the Permissions window is open with Accessibility not granted
- **WHEN** the user grants Accessibility in System Settings
- **THEN** within 1 s the Accessibility row shows Granted

#### Scenario: Continue waits for every permission
- **GIVEN** the Permissions window is open with one permission not granted
- **WHEN** the user looks at the footer
- **THEN** Continue is disabled

#### Scenario: Continue closes the window once all are granted
- **GIVEN** all three permissions are granted and the window is open
- **WHEN** the user presses Continue
- **THEN** the window closes and the dictation pipeline is running

## ADDED Requirements

### Requirement: A switch that does not take is explained
After the user opened System Settings from a row and the permission still is not granted, the row SHALL say that when voicepuncher is already switched on there the entry belongs to an earlier build and must be removed with − and added again with +.

#### Scenario: Accessibility switched on for an earlier build
- **GIVEN** the user pressed Open System Settings in the Accessibility row
- **AND** Accessibility is still not granted
- **WHEN** the Permissions window checks again
- **THEN** the Accessibility row says to remove voicepuncher with − and add it again with + when it is already switched on

#### Scenario: No hint before System Settings was opened
- **GIVEN** Accessibility is not granted
- **AND** the user has not opened System Settings from the window
- **WHEN** the Permissions window shows
- **THEN** the Accessibility row shows no hint

### Requirement: Input Monitoring can be asked for again after a denial
Open System Settings in the Input Monitoring row of a denied permission SHALL ask the system for Input Monitoring again, open the Input Monitoring pane and, when the app runs from its bundle, show the bundle in Finder, so the user can add it with + when the system did not list it; the row SHALL then say to add voicepuncher with + when it is not in the list.

#### Scenario: Input Monitoring denied
- **GIVEN** Input Monitoring is not granted and the app runs from its bundle
- **WHEN** the user presses Open System Settings in the Input Monitoring row
- **THEN** the system is asked for Input Monitoring again
- **AND** System Settings opens on the Input Monitoring page
- **AND** Finder shows the voicepuncher bundle
- **AND** the row says to add voicepuncher with + when it is not in the list
