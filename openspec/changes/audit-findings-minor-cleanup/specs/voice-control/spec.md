## MODIFIED Requirements

### Requirement: A command said while the command spotting service was away still counts
The system SHALL send the service the speech heard during the last 10 seconds of an outage once the service is back, so a command said in that time is carried out late rather than lost. Microphone audio that never reached the app before the outage SHALL NOT shift the speech sent again: it still begins before the moment the service went away.

#### Scenario: "stop dictation" said during a short outage ends the dictation
- **GIVEN** a dictation runs
- **AND** the command spotting service is away for 3 seconds
- **WHEN** the user says "stop dictation" during those 3 seconds
- **THEN** the dictation ends once the service is back, and the words "stop dictation" are not in the pasted text

#### Scenario: Speech sent again after an outage begins before the loss when microphone audio was lost earlier
- **GIVEN** listening is on and some seconds of microphone audio never reached the app
- **AND** the command spotting service went away afterwards
- **WHEN** the service is back
- **THEN** the speech sent to it again begins before the moment the service went away
