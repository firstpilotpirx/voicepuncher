# menu-bar-presence Specification

## Purpose
The application's presence in the macOS menu bar: a status item whose icon shows what the app is doing, and a menu that tells the state in words and gives Settings, the logs folder and Quit, so the app is visible, understandable and quittable without a terminal.

## Requirements

### Requirement: The menu bar shows the app from launch
From launch until it quits, the app SHALL show a status item in the system menu bar. While the runtime starts or restarts the icon SHALL be the waiting icon (an hourglass); once the runtime is ready and no dictation runs, the ready icon (a microphone outline); once the runtime failed, the failed icon (a filled warning triangle) until the app exits, as it does when a local service does not answer at launch. The icon SHALL follow the light and the dark menu bar appearance.

#### Scenario: The status item appears while the app starts
- **GIVEN** the app was just launched and its runtime is still starting
- **WHEN** the menu bar is looked at
- **THEN** the status item shows the waiting icon

#### Scenario: The ready icon shows once the runtime is ready
- **GIVEN** the status item shows the waiting icon
- **WHEN** the runtime becomes ready
- **THEN** the status item shows the ready icon

#### Scenario: A restart shows the waiting icon and ready again afterwards
- **GIVEN** the runtime is ready and no dictation runs
- **WHEN** the runtime restarts with a new configuration and becomes ready again
- **THEN** the status item shows the waiting icon during the restart and the ready icon afterwards

#### Scenario: A failed runtime shows the failed icon until the app exits
- **GIVEN** the runtime is starting
- **WHEN** the runtime fails because a local service does not answer
- **THEN** the status item shows the failed icon and the app exits as before

### Requirement: The icon mirrors the dictation
While the runtime is ready, the icon SHALL mirror the stage of the dictation as the overlay shows it: recording — a filled microphone with a red dot; recognizing speech and refining text — a waveform; waiting for a text field — a keyboard; pasted — the ready icon again; a failure notice — a warning triangle for as long as the notice shows, then the ready icon. A command sign SHALL leave the icon as it is.

#### Scenario: A recording shows the recording icon
- **GIVEN** the runtime is ready
- **WHEN** a recording starts
- **THEN** the status item shows the recording icon

#### Scenario: The icon follows every stage of a dictation
- **GIVEN** text refinement is enabled and a button has keyboard focus
- **WHEN** the user records a dictation with speech in it, ends it and then clicks into a text field
- **THEN** the status item shows the recording icon, then the waveform, then the keyboard, then the ready icon after the paste

#### Scenario: A failure notice shows the warning icon until it hides
- **GIVEN** the speech recognition service is not running
- **WHEN** the user stops a dictation
- **THEN** the status item shows the warning icon while the failure notice shows and the ready icon once it hides

#### Scenario: A command sign leaves the icon as it was
- **GIVEN** voice control is turned on and a recording runs
- **WHEN** the user says "clear text"
- **THEN** the status item keeps showing the recording icon

### Requirement: The icon shows voice control
While voice control listens and no dictation runs, the icon SHALL be an ear instead of the ready icon. While the commands-unavailable sign shows and no dictation runs, the icon SHALL be an ear with a warning badge; a dictation in progress SHALL show its own icon instead, and the ear with the badge SHALL come back after it while the sign still shows.

#### Scenario: Voice control listening shows the listening icon
- **GIVEN** voice control is turned on and no dictation runs
- **WHEN** the runtime becomes ready
- **THEN** the status item shows the listening icon instead of the ready icon

#### Scenario: Commands unavailable shows the warning badge on the ear
- **GIVEN** voice control is turned on and no dictation runs
- **WHEN** the command spotting service has been away long enough for the commands-unavailable sign
- **THEN** the status item shows the ear with a warning badge until the sign goes

#### Scenario: A dictation outranks the commands-unavailable sign
- **GIVEN** the commands-unavailable sign shows
- **WHEN** a recording starts
- **THEN** the status item shows the recording icon

### Requirement: The menu tells the state in words
Clicking the status item SHALL open a menu whose first line is the app's name and version and whose second line names the current state in plain words: ready, with the hotkey by its human name and how to use it in the configured recording mode; recording, with how to stop; recognizing speech; refining text; waiting for a text field; voice control listening; voice control commands unavailable; starting; restarting; failed, with the reason. Both lines SHALL be plain text that cannot be chosen.

#### Scenario: The ready status line names the hotkey
- **GIVEN** the runtime is ready, the hotkey is the right Option key and the recording mode is Hold
- **WHEN** the user opens the menu
- **THEN** the status line reads "Ready — hold Right Option to dictate"

#### Scenario: The ready status line in Toggle mode says press
- **GIVEN** the runtime is ready, the hotkey is the right Option key and the recording mode is Toggle
- **WHEN** the user opens the menu
- **THEN** the status line reads "Ready — press Right Option to dictate"

#### Scenario: The recording status line says how to stop
- **GIVEN** a recording runs, the hotkey is the right Option key and the recording mode is Hold
- **WHEN** the user opens the menu
- **THEN** the status line reads "Recording — release Right Option to stop"

#### Scenario: The failed status line gives the reason
- **GIVEN** the runtime failed because the speech recognition service does not answer
- **WHEN** the user opens the menu
- **THEN** the status line starts with "Failed:" and names the speech recognition service

#### Scenario: The status line reflects the state at the moment the menu opens
- **GIVEN** the menu was opened and closed while the runtime was ready
- **WHEN** the user opens the menu again during speech recognition
- **THEN** the status line reads "Recognizing speech…"

#### Scenario: Every hotkey has a human name
- **GIVEN** the runtime is ready
- **WHEN** the user opens the menu with any configurable hotkey
- **THEN** the status line names that key as a person would say it, such as "Right Command", "Fn", "Caps Lock", "Option+E", "Cmd+E" or "F13"

### Requirement: Quit ends the application
The menu SHALL offer Quit with the standard shortcut. Choosing it SHALL stop the runtime and end the process with exit code 0 within a moment, without waiting for a dictation: a recording in progress is cancelled with the cancel sound, as a cancelled dictation is today, and nothing of it is pasted.

#### Scenario: Quit while idle ends the process cleanly
- **GIVEN** the runtime is ready and no dictation runs
- **WHEN** the user chooses Quit in the menu
- **THEN** the runtime stops and the process ends with exit code 0

#### Scenario: Quit during a recording cancels it
- **GIVEN** a recording runs
- **WHEN** the user chooses Quit in the menu
- **THEN** the recording is cancelled with the cancel sound, nothing is pasted and the process ends with exit code 0

### Requirement: The logs folder opens from the menu
The menu SHALL offer Open Logs Folder. Choosing it SHALL show the app's logs folder in the file browser, creating the folder first when it does not exist yet.

#### Scenario: Open Logs Folder shows the folder
- **GIVEN** the logs folder exists
- **WHEN** the user chooses Open Logs Folder
- **THEN** the file browser shows the logs folder

#### Scenario: Open Logs Folder creates a missing folder
- **GIVEN** the logs folder does not exist
- **WHEN** the user chooses Open Logs Folder
- **THEN** the logs folder exists and the file browser shows it

### Requirement: The app has no Dock icon
The app SHALL run as a menu bar application: no Dock icon and no application menu, whether it runs from its bundle or as a bare binary from a terminal.

#### Scenario: The bare binary shows no Dock icon
- **GIVEN** the app is started from a terminal
- **WHEN** it is running
- **THEN** the Dock shows no icon for it and the status item is in the menu bar

### Requirement: Settings open the settings window from the menu
The menu SHALL offer Settings with the standard shortcut. Choosing it SHALL open the settings window.

#### Scenario: Settings opens the settings window
- **GIVEN** the app runs with its configuration file
- **WHEN** the user chooses Settings
- **THEN** the settings window opens in front

### Requirement: The menu shows the wait for permissions
While the runtime waits for permissions, the status item SHALL show the waiting icon and the menu's status line SHALL read "Waiting for permissions — open Permissions…".

#### Scenario: A missing permission at launch
- **GIVEN** Accessibility is not granted
- **WHEN** the app launches and the user opens the menu
- **THEN** the status item shows the waiting icon
- **AND** the status line reads "Waiting for permissions — open Permissions…"

### Requirement: Permissions open from the menu
The menu SHALL offer Permissions… under the status line in every runtime state. Choosing it SHALL bring the Permissions window to the front with the current statuses.

#### Scenario: Permissions while every permission is granted
- **GIVEN** the runtime is Ready
- **WHEN** the user chooses Permissions… in the menu
- **THEN** the Permissions window opens with all three rows marked Granted
