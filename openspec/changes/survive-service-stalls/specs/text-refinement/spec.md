## ADDED Requirements

### Requirement: A dropped connection to the text refinement service is retried once
The system SHALL send the refinement request once more when the text refinement service refuses or drops the connection before it answers, and SHALL log the dropped attempt; only when the second attempt fails too does refinement fall back to the raw transcript.

#### Scenario: The text refinement service drops one connection
- **GIVEN** the text refinement service drops the first connection and then answers normally
- **WHEN** the user finishes a recording
- **THEN** the refined text is pasted and the dropped attempt is logged

#### Scenario: The text refinement service keeps dropping the connection
- **GIVEN** the text refinement service drops every connection
- **WHEN** the user finishes a recording
- **THEN** the request is sent twice in total and the raw transcript is pasted
