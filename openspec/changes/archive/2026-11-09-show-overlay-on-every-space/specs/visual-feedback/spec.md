## MODIFIED Requirements

### Requirement: The overlay appears without rebuilding its windows
The system SHALL keep the overlay's windows from one showing to the next and build them again only when the screens change or when the pill is not on the desktop the user is on, so the overlay is drawn within 0.05 s of being asked to show and appears on every screen of the moment and on the current desktop. While the overlay shows a state, a switch to another desktop SHALL bring it there the same way.

#### Scenario: The recording sign is drawn at once
- **GIVEN** the app runs and the overlay is hidden
- **WHEN** a recording starts
- **THEN** the recording sign is drawn within 0.05 s

#### Scenario: A screen added later gets the overlay
- **GIVEN** the overlay has been shown on one screen
- **AND** a second screen was connected since
- **WHEN** a recording starts
- **THEN** the overlay's windows are built again for the screens now connected
- **AND** its recording sign is drawn

#### Scenario: The overlay shows on the desktop the user switched to
- **GIVEN** the overlay has been shown on the first desktop
- **AND** the user switched to another desktop
- **WHEN** a recording starts
- **THEN** the recording sign shows on the desktop the user is on

#### Scenario: Windows dropped from the current desktop are built again on show
- **GIVEN** the overlay has shown once and is hidden
- **AND** its windows are no longer on the current desktop
- **WHEN** a recording starts
- **THEN** the overlay's windows are built again and show the recording sign on the current desktop

#### Scenario: A desktop switch during a recording brings the overlay along
- **GIVEN** the overlay shows the recording
- **AND** its windows are no longer on the current desktop
- **WHEN** the user switches desktop
- **THEN** the overlay's windows are built again and show the recording sign
