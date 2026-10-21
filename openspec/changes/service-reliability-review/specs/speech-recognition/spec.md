## ADDED Requirements

### Requirement: A chunk waits for its service to come back
When the attempts for a dropped connection are used up and no copy of the speech recognition service accepts connections, the system SHALL wait up to 30 seconds for a copy to come back, log the wait, and send the chunk again with a fresh set of attempts; only when none comes back is the chunk lost. Copies that accept connections and drop them SHALL NOT be waited for.

#### Scenario: Every copy of the dictation service is killed while a chunk is sent
- **GIVEN** the app started the speech recognition service for dictation
- **WHEN** every copy of it is killed while a dictation's chunk is being transcribed
- **THEN** the chunk's text comes back once the app has started the copies again, and no failure is reported
- **AND** the wait for the service is logged

#### Scenario: The service does not come back
- **GIVEN** no copy of the speech recognition service accepts connections, and none comes back
- **WHEN** a chunk is sent
- **THEN** the chunk is lost once the wait for the service ran out, and the wait and the loss are logged
