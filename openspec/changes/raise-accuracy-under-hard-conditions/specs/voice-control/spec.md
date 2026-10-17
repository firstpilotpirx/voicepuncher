## MODIFIED Requirements

### Requirement: A slow command spotting service does not stop listening
The system SHALL keep listening while the command spotting service answers late, however long its spotting results take, as long as the service still answers on its connection; only a dropped connection or a service that answers nothing at all for 5 seconds SHALL count as the service going away.

#### Scenario: A confirmation that takes longer than 10 seconds keeps listening on
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 15 seconds to answer one confirmation of the command spotting service
- **WHEN** the user keeps dictating through those 15 seconds and then says "stop dictation"
- **THEN** listening is still on and no error about the command spotting service is logged
- **AND** the dictation ends and its text is pasted
