## MODIFIED Requirements

### Requirement: The menu shows the wait for permissions
While the runtime waits for permissions, the status item SHALL show an icon of its own for the missing permission, distinct from the waiting icon of the other waits, and the menu's status line SHALL read "Waiting for <permissions>…", naming each missing permission, and open the Permissions window when chosen.

#### Scenario: A missing permission at launch
- **GIVEN** Accessibility is not granted
- **WHEN** the app launches and the user opens the menu
- **THEN** the status item shows the missing-permission icon, not the waiting icon
- **AND** the status line reads "Waiting for Accessibility…"

#### Scenario: Two missing permissions
- **GIVEN** Accessibility and Input Monitoring are not granted
- **WHEN** the user opens the menu
- **THEN** the status line reads "Waiting for Accessibility and Input Monitoring…"

#### Scenario: The waiting item opens the Permissions window
- **GIVEN** the runtime waits for Accessibility
- **WHEN** the user chooses "Waiting for Accessibility…" in the menu
- **THEN** the Permissions window comes to the front

### Requirement: The menu tells the state in words
Clicking the status item SHALL open a menu whose first line is the app's name and version and whose second line names the current state in plain words: ready, with the hotkey by its human name and how to use it in the configured recording mode; recording, with how to stop; recognizing speech; refining text; waiting for a text field; waiting for the missing permissions, named; voice control listening; voice control commands unavailable; starting; restarting; failed, with the reason. Both lines SHALL be plain text that cannot be chosen, except the status line while the runtime waits for permissions, which opens the Permissions window.

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
