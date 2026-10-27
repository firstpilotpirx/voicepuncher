## ADDED Requirements

### Requirement: The overlay appears without rebuilding its windows
The system SHALL keep the overlay's windows from one showing to the next and build them again only when the screens change, so the overlay is drawn within 0.05 s of being asked to show and appears on every screen of the moment.

#### Scenario: The recording sign is drawn at once
- **GIVEN** the app runs and the overlay is hidden
- **WHEN** a recording starts
- **THEN** the recording sign is drawn within 0.05 s

#### Scenario: A screen added later gets the overlay
- **GIVEN** the overlay has been shown on one screen
- **AND** a second screen was connected since
- **WHEN** a recording starts
- **THEN** the overlay is drawn on the screen it belongs to among the two
