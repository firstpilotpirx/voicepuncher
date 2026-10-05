## ADDED Requirements

### Requirement: Transcript after a long pause is refined
The system SHALL send a transcript to the text refinement service on a connection the service has not closed, whatever time passed since the previous refinement, so that a long pause between recordings does not make the raw transcript be pasted.

#### Scenario: Service closed the idle connection during a long pause
- **GIVEN** a transcript was refined
- **AND** the text refinement service has since reset the connection that refinement used
- **WHEN** the next transcript is refined
- **THEN** the refined text from the service is returned, not the raw transcript
