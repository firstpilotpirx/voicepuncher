## ADDED Requirements

### Requirement: Listening comes back when the microphone stalls
While listening, when the microphone used for voice commands reports an error or delivers no audio for 2 seconds (5 seconds before its first audio), the system SHALL log it once and open the same input device again, retrying every second until it opens, then go on listening for the same commands without an app restart. The silent time SHALL be streamed as silence, so later audio keeps its capture times.

#### Scenario: A microphone that stops delivering audio is opened again
- **GIVEN** listening is on and the microphone delivered audio
- **WHEN** the microphone stops delivering audio without reporting an error
- **THEN** within 3 seconds the app logs that the microphone stalled and opens it again
- **AND** the audio of the reopened microphone reaches the command spotting service

#### Scenario: A microphone that reports an error is opened again
- **GIVEN** listening is on
- **WHEN** the microphone reports an error
- **THEN** the app opens it again

#### Scenario: A microphone that cannot be opened again is retried
- **GIVEN** the microphone stalled
- **WHEN** opening it fails twice and then succeeds
- **THEN** the app keeps trying every second until it is open, logging the failure once

#### Scenario: The silent time keeps the capture times
- **GIVEN** the microphone was silent for 3 seconds before it was opened again
- **WHEN** the reopened microphone delivers audio
- **THEN** that audio is streamed after 3 seconds of silence, so its time in the stream is its capture time
