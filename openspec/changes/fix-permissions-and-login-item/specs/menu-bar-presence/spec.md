## MODIFIED Requirements

### Requirement: The menu shows the wait for permissions
While the runtime waits for permissions, the status item SHALL show an icon of its own for the missing permission, distinct from the waiting icon of the other waits, and the menu's first item SHALL read "Waiting for <permissions>…", naming each missing permission, and open the Permissions window when chosen.

#### Scenario: A missing permission at launch
- **GIVEN** Accessibility is not granted
- **WHEN** the app launches and the user opens the menu
- **THEN** the status item shows the missing-permission icon, not the waiting icon
- **AND** the menu's first item reads "Waiting for Accessibility…"

#### Scenario: Two missing permissions
- **GIVEN** Accessibility and Input Monitoring are not granted
- **WHEN** the user opens the menu
- **THEN** the menu's first item reads "Waiting for Accessibility and Input Monitoring…"

#### Scenario: The waiting item opens the Permissions window
- **GIVEN** the runtime waits for Accessibility
- **WHEN** the user chooses "Waiting for Accessibility…" in the menu
- **THEN** the Permissions window comes to the front
