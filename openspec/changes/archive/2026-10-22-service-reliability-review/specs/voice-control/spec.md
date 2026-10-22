## ADDED Requirements

### Requirement: Listening moves to another copy of the command spotting service
When the copy of the command spotting service the app streams to goes away while another copy answers, the system SHALL connect to that other copy at its first attempt to reach the service again and replay the speech the lost copy may have missed, so a command said meanwhile still counts and the overlay never shows that voice commands do not work.

#### Scenario: The command spotting copy the app listens with is killed
- **GIVEN** the app started the command spotting service as two copies and listening is on
- **WHEN** the copy the app streams to is killed and the user says "press enter" a moment later
- **THEN** the Return key is pressed in the focused app
- **AND** the overlay did not show that voice commands do not work
- **AND** the killed copy answers again within 30 seconds

### Requirement: A confirmation refused by its copy goes to another copy
The command spotting service SHALL send a confirmation whose copy of the speech recognition service for commands refused the connection to another copy that is up, within the confirmation's deadline, and SHALL log the refused attempt.

#### Scenario: A copy for commands is down as a confirmation reaches it
- **GIVEN** the speech recognition service for commands runs as two copies, and one stopped accepting connections a moment ago
- **WHEN** a confirmation is sent to that copy
- **THEN** the other copy confirms it, and the refused attempt is logged

#### Scenario: A copy for commands is killed while voice control runs
- **GIVEN** the app started the speech recognition service for commands as its copies and listening is on
- **WHEN** one copy is killed and the user says "press enter" right after
- **THEN** the Return key is pressed in the focused app
