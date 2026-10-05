## ADDED Requirements

### Requirement: Recording after a long pause reaches the service at once
The system SHALL send a recording to the speech recognition service on a connection the service has not closed, whatever time passed since the previous recording, so that the first attempt reaches the service and no dropped attempt is logged.

#### Scenario: Service closed the idle connection during a long pause
- **GIVEN** a recording was transcribed
- **AND** the speech recognition service has since reset the connection that recording used
- **WHEN** the user finishes the next recording
- **THEN** its transcript is passed on for refinement and pasting, and no dropped attempt is logged
