# permissions-gate Specification

## Purpose
The three macOS permissions voicepuncher needs — Microphone, Accessibility and Input Monitoring — are checked at every launch and while the app runs; the user is told which are missing and taken to the exact System Settings page, dictation starts only with all three and stops when one is revoked.

## Requirements

### Requirement: Dictation starts only with every permission granted
In hotkey mode the app SHALL check Microphone, Accessibility and Input Monitoring at launch and start the dictation pipeline only when all three are granted. While any is missing, the app SHALL keep running without a pipeline, open the Permissions window, and start the pipeline by itself, without a relaunch, once the last missing permission is granted.

#### Scenario: Launch with every permission granted
- **GIVEN** Microphone, Accessibility and Input Monitoring are granted
- **WHEN** the app launches in hotkey mode
- **THEN** the dictation pipeline starts
- **AND** the Permissions window does not open

#### Scenario: Launch with a permission missing
- **GIVEN** Accessibility is not granted and the other two are
- **WHEN** the app launches in hotkey mode
- **THEN** no dictation pipeline runs
- **AND** the Permissions window opens with the Accessibility row marked not granted
- **AND** the process keeps running

#### Scenario: The last grant starts dictation
- **GIVEN** the app runs without a pipeline because Input Monitoring is missing
- **WHEN** the user grants Input Monitoring
- **THEN** within 2 s the dictation pipeline starts in the same process

### Requirement: Each permission is shown with its status and its remedy
The Permissions window SHALL show one row per permission with its name, one line of why the app needs it, its status — Granted, Not granted, Not asked yet, or Checking… while a request is pending — and one button: Request… for a permission not asked yet, Open System Settings for one not granted, none for one granted.

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

### Requirement: Not asked yet differs from denied
The app SHALL tell a permission never requested from one the user denied or revoked, without opening a device to find out, and SHALL offer Request… for the first and Open System Settings for the second.

#### Scenario: A permission never requested
- **GIVEN** the app never asked for Microphone
- **WHEN** the Permissions window shows
- **THEN** the Microphone row shows Not asked yet with Request…
- **AND** the microphone in-use indicator does not light up

#### Scenario: A permission denied
- **GIVEN** the user denied Microphone
- **WHEN** the Permissions window shows
- **THEN** the Microphone row shows Not granted with Open System Settings

### Requirement: A revoked permission stops dictation
While the pipeline runs the app SHALL re-check the permissions at least every 5 s. When one is no longer granted, it SHALL stop the pipeline, cancel a dictation in progress with the cancel sound and a failure notice naming the revoked permission, open the Permissions window with that row marked, and start the pipeline again once it is granted again.

#### Scenario: Accessibility revoked while idle
- **GIVEN** the pipeline runs and no dictation is in progress
- **WHEN** the user removes Accessibility in System Settings
- **THEN** within 5 s the pipeline stops
- **AND** the Permissions window opens with the Accessibility row marked not granted and a line saying Accessibility was revoked and dictation stopped

#### Scenario: Microphone revoked during a recording
- **GIVEN** a recording is in progress
- **WHEN** the user removes Microphone in System Settings
- **THEN** within 5 s the recording is cancelled with the cancel sound and nothing is pasted
- **AND** a failure notice says the Microphone permission was revoked

#### Scenario: Granting a revoked permission again resumes dictation
- **GIVEN** the pipeline stopped because Accessibility was revoked
- **WHEN** the user grants Accessibility again
- **THEN** the dictation pipeline starts again without a relaunch

### Requirement: The window names who must be granted
When the app runs as a bare binary from another application, the Permissions window SHALL name that application as the one to grant the permissions to, because macOS attributes them to it; when the app runs from its bundle, the window SHALL name voicepuncher and no other application.

#### Scenario: Run from a terminal
- **GIVEN** the app runs as a bare binary started from iTerm2
- **WHEN** the Permissions window shows
- **THEN** it says voicepuncher runs from iTerm2 and the permissions must be granted to iTerm2

#### Scenario: Run from the bundle
- **GIVEN** the app runs from its bundle
- **WHEN** the Permissions window shows
- **THEN** it names no other application to grant the permissions to

### Requirement: The window is current and can be opened at any time
The Permissions window SHALL re-check every permission at least every 2 s while it is open and when the app becomes active, and update its rows. Continue SHALL be enabled only when all three are granted and SHALL close the window; Quit SHALL end the app.

#### Scenario: A grant in System Settings shows up in the open window
- **GIVEN** the Permissions window is open with Accessibility not granted
- **WHEN** the user grants Accessibility in System Settings
- **THEN** within 2 s the Accessibility row shows Granted

#### Scenario: Continue waits for every permission
- **GIVEN** the Permissions window is open with one permission not granted
- **WHEN** the user looks at the footer
- **THEN** Continue is disabled

#### Scenario: Continue closes the window once all are granted
- **GIVEN** all three permissions are granted and the window is open
- **WHEN** the user presses Continue
- **THEN** the window closes and the dictation pipeline is running

### Requirement: Permission changes are logged
The app SHALL log the status of all three permissions at launch and at every change, in one line naming each permission and its status.

#### Scenario: A change is logged
- **GIVEN** the app runs with Microphone granted, Accessibility not granted and Input Monitoring not asked yet
- **WHEN** the user grants Accessibility
- **THEN** the log has one line naming Microphone granted, Accessibility granted and Input Monitoring not asked yet

### Requirement: File mode needs no permission
Transcribing a recording file SHALL run without checking any permission and without opening the Permissions window.

#### Scenario: File transcription without permissions
- **GIVEN** no permission is granted
- **WHEN** the user transcribes a recording file
- **THEN** the transcript is printed
- **AND** no Permissions window opens
