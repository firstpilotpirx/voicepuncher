## MODIFIED Requirements

### Requirement: Dropped connection to the service is retried
The system SHALL send the recording again, up to 3 more times with a growing pause of at most 1.5 seconds in total, when the speech recognition service refuses or drops the connection before it answers, whatever connection error the operating system reports for the drop, and SHALL log each dropped attempt; only when every attempt fails is the cycle dropped as a service failure. An error answer, an unusable answer or a timeout SHALL NOT be retried.

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
- **THEN** the recording is sent once and the cycle is dropped as a service failure
