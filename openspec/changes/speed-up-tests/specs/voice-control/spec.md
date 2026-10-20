# Spec Delta

## MODIFIED Requirements

### Requirement: Listening comes back when the command spotting service is back
When the command spotting service answers again after it went away, the system SHALL resume listening for the commands of the current state within 5 seconds and log that listening is back, without a restart of the app and without playing a sound. The system SHALL NOT give up reaching for the service however many attempts fail: an outage that outlasts the speech it replays and the tenth failed attempt still ends with listening back.

#### Scenario: Commands work again after the command spotting service restarts
- **GIVEN** listening is on and no dictation runs
- **AND** the command spotting service went away and was started again
- **WHEN** the user says "press enter" 5 seconds after the service answers again
- **THEN** the Return key is pressed in the focused app
- **AND** no listening sound played after the app started

#### Scenario: Listening is not given up after many failed attempts
- **GIVEN** listening is on
- **AND** the command spotting service stays away past the speech the app replays and past the tenth failed attempt to reach it
- **WHEN** the service answers again
- **THEN** listening is on again within 5 seconds
- **AND** the first failed attempt and the tenth were logged, and the service was reported still away once

### Requirement: A slow command spotting service does not stop listening
The system SHALL keep listening while the command spotting service answers late, however long its spotting results take, as long as the service still answers on its connection; only a dropped connection or a service that answers nothing at all for 5 seconds after the app asked SHALL count as the service going away. A pause of the app's own, during which it reads nothing from the service, SHALL NOT count as the service's silence.

#### Scenario: A confirmation that takes longer than 10 seconds keeps listening on
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands holds one confirmation of the command spotting service for longer than the service waits for a confirmation
- **WHEN** the user keeps dictating through that hold and then says "stop dictation"
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
