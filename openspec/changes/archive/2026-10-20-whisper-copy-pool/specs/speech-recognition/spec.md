# Spec Delta

## MODIFIED Requirements

### Requirement: Dropped connection to the service is retried
The system SHALL send the recording again, up to 3 more times with a growing pause of at most 1.5 seconds in total, when the speech recognition service refuses or drops the connection before it answers, whatever connection error the operating system reports for the drop, and SHALL log each dropped attempt; each new attempt SHALL go to another copy of the service when one is up. Only when every attempt fails is the cycle dropped as a service failure. An unusable answer SHALL NOT be retried; a timeout or an error answer is retried as its own requirement states.

#### Scenario: Service is briefly busy
- **GIVEN** the speech recognition service drops the first two connections and then answers normally
- **WHEN** the user finishes a recording
- **THEN** the transcript the service returns is passed on for refinement and pasting, and each of the two dropped attempts is logged

#### Scenario: Drop reported as an invalid argument
- **GIVEN** the speech recognition service drops the connection after the recording was sent
- **AND** the operating system reports the drop as an invalid argument instead of a reset
- **WHEN** the user finishes a recording
- **THEN** the recording is sent again, as for any dropped connection

#### Scenario: Service keeps dropping the connection
- **GIVEN** the speech recognition service drops every connection
- **WHEN** the user finishes a recording
- **THEN** the recording is sent 4 times in total and then the cycle is dropped as a service failure

#### Scenario: Service answers with an error
- **GIVEN** the speech recognition service answers every request with an error
- **WHEN** the user finishes a recording
- **THEN** the recording is sent 3 times in total and then the cycle is dropped as a service failure

#### Scenario: A chunk whose copy is killed is transcribed by another copy
- **GIVEN** the speech recognition service for dictation runs as several copies
- **WHEN** the copy that is to transcribe a chunk is killed as the chunk reaches it
- **THEN** that chunk is transcribed by another copy, and the killed copy gets no second attempt

### Requirement: A request that times out or gets an error answer is retried
The system SHALL send a recording or chunk again, up to 2 more times, when the speech recognition service does not answer it in time or answers with an error, and SHALL log each failed attempt with how long it took; an attempt after one the app gave up on SHALL go to another copy than the one still working on the abandoned attempt, or wait for a free copy, never queue behind the abandoned attempt.

#### Scenario: The first request of a chunk hangs
- **GIVEN** the speech recognition service never answers the first request for a chunk and answers the next one normally
- **WHEN** the user finishes the dictation
- **THEN** the chunk's text is in the pasted text, and the hung attempt is logged

#### Scenario: One error answer is retried
- **GIVEN** the speech recognition service answers the first request with an error and the next one normally
- **WHEN** the user finishes a recording
- **THEN** the transcript is pasted and the error answer is logged

#### Scenario: The attempt after a hung one goes to another copy
- **GIVEN** the speech recognition service runs as two copies, and one copy never answers the first request for a recording
- **WHEN** the user finishes the recording
- **THEN** the other copy answers the next attempt, and the transcript is pasted
