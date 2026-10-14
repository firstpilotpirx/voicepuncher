## MODIFIED Requirements

### Requirement: Service failure during a cycle drops only that cycle
The system SHALL drop the current recording, log the failure and stay ready for the next recording when the speech recognition service fails, times out or returns an unusable answer on every attempt the retry rules allow.

#### Scenario: Service stops between recordings
- **WHEN** the user finishes a recording after the speech recognition service has stopped
- **THEN** nothing is pasted, the failure is logged, and the app accepts the next recording

### Requirement: Dropped connection to the service is retried
The system SHALL send the recording again, up to 3 more times with a growing pause of at most 1.5 seconds in total, when the speech recognition service refuses or drops the connection before it answers, whatever connection error the operating system reports for the drop, and SHALL log each dropped attempt; only when every attempt fails is the cycle dropped as a service failure. An unusable answer SHALL NOT be retried; a timeout or an error answer is retried as its own requirement states.

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

## ADDED Requirements

### Requirement: A request that times out or gets an error answer is retried
The system SHALL send a recording or chunk again, up to 2 more times, when the speech recognition service does not answer it in time or answers with an error, and SHALL log each failed attempt with how long it took; the service SHALL stop working on a request the app gave up on.

#### Scenario: The first request of a chunk hangs
- **GIVEN** the speech recognition service never answers the first request for a chunk and answers the next one normally
- **WHEN** the user finishes the dictation
- **THEN** the chunk's text is in the pasted text, and the hung attempt is logged

#### Scenario: One error answer is retried
- **GIVEN** the speech recognition service answers the first request with an error and the next one normally
- **WHEN** the user finishes a recording
- **THEN** the transcript is pasted and the error answer is logged

### Requirement: The wait for a transcript follows the length of the audio
The system SHALL give up on one request after a time that grows with the length of the audio sent, a few seconds for a short recording and never more than the configured request timeout, so a request that hangs holds the dictation for seconds rather than a minute.

#### Scenario: A hung short chunk is given up quickly
- **GIVEN** the speech recognition service never answers any request
- **WHEN** the user finishes a 5-second recording
- **THEN** the cycle is dropped as a service failure within 30 seconds of the stop, after every attempt was logged

### Requirement: A lost chunk does not lose the rest of the dictation
The system SHALL paste the text of every chunk of a dictation that was transcribed when one chunk fails on every attempt, and SHALL log which chunk was lost.

#### Scenario: The middle chunk of a long dictation fails
- **GIVEN** a dictation long enough for three chunks
- **AND** the speech recognition service fails every attempt for the second chunk
- **WHEN** the user finishes the dictation
- **THEN** the text of the first and third chunks is pasted, and the lost second chunk is logged
