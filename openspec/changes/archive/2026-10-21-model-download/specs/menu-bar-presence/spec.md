## ADDED Requirements

### Requirement: The menu shows the wait for models
While the runtime waits for models, the status item SHALL show the waiting icon and the menu's status line SHALL read "Waiting for models — open Download models…".

#### Scenario: A missing model at launch
- **GIVEN** the dictation model is missing
- **WHEN** the app launches and the user opens the menu
- **THEN** the status item shows the waiting icon
- **AND** the status line reads "Waiting for models — open Download models…"

### Requirement: Download models opens from the menu
When the app manages its services, the menu SHALL offer Download models… under Permissions… in every runtime state. Choosing it SHALL bring the Download window to the front with the current status of every needed file.

#### Scenario: Download models while every model is present
- **GIVEN** the runtime is Ready
- **WHEN** the user chooses Download models… in the menu
- **THEN** the Download window opens with every row marked Present and Done enabled
