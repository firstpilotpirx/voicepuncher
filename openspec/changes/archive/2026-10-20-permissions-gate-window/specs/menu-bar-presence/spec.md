# Spec Delta

## ADDED Requirements

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
