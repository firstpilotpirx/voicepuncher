# Spec Delta

## MODIFIED Requirements

### Requirement: The wait for a transcript follows the length of the audio
The system SHALL give up on one request after a time that grows with the length of the audio sent, a few seconds for a short recording and never more than the configured request timeout, so a request that hangs holds the dictation for seconds rather than a minute; after the last attempt the cycle is dropped at once.

#### Scenario: A hung short chunk is given up quickly
- **GIVEN** the speech recognition service never answers any request
- **WHEN** the user finishes a 5-second recording
- **THEN** the cycle is dropped as a service failure after three attempts, each given up within the wait for a short recording, and every attempt was logged
