## MODIFIED Requirements

### Requirement: A slow command spotting service does not stop listening
The system SHALL keep listening while the command spotting service answers late, however long its spotting results take, as long as the service still answers on its connection; only a dropped connection or a service that answers nothing at all for 5 seconds after the app asked SHALL count as the service going away. A pause of the app's own, during which it reads nothing from the service, SHALL NOT count as the service's silence.

#### Scenario: A confirmation that takes longer than 10 seconds keeps listening on
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 15 seconds to answer one confirmation of the command spotting service
- **WHEN** the user keeps dictating through those 15 seconds and then says "stop dictation"
- **THEN** listening is still on and no error about the command spotting service is logged
- **AND** the dictation ends and its text is pasted

#### Scenario: A pause of the app's own does not lose the service
- **GIVEN** listening is on and the command spotting service answers every check at once
- **WHEN** the app reads nothing from the service for 6 seconds, as when it is busy or the computer sleeps
- **THEN** the service is not reported lost and no error about the command spotting service is logged

#### Scenario: A burst of audio after long listening does not lose the service
- **GIVEN** listening is on with a freshly started command spotting service and the machine is busy
- **WHEN** 30 seconds of silence followed by "start dictation" reach the app at once
- **THEN** the service is not reported lost
- **AND** "start dictation" is recognized
