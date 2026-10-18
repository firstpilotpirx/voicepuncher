# Spec Delta

## MODIFIED Requirements

### Requirement: The app keeps reaching for the command spotting service while it is away
When the command spotting service stops answering while listening, the system SHALL log an error naming the service and keep trying to reach the service, with a growing pause of at most 4 seconds, until it answers or the app exits; it SHALL log the first failed attempt and every tenth, so a long outage does not flood the log. The loss SHALL play no sound, since listening stays on.

#### Scenario: The app keeps reaching for the command spotting service when it goes away
- **GIVEN** listening is on
- **WHEN** the command spotting service stops answering
- **THEN** an error naming the command spotting service is logged
- **AND** no sound plays
- **AND** the app tries to reach the service again, logging its first failed attempt and every tenth

### Requirement: Listening comes back when the command spotting service is back
When the command spotting service answers again after it went away, the system SHALL resume listening for the commands of the current state within 5 seconds and log that listening is back, without a restart of the app and without playing a sound.

#### Scenario: Commands work again after the command spotting service restarts
- **GIVEN** listening is on and no dictation runs
- **AND** the command spotting service went away and was started again
- **WHEN** the user says "press enter" 5 seconds after the service answers again
- **THEN** the Return key is pressed in the focused app
- **AND** no listening sound played after the app started

#### Scenario: Listening is not given up after many failed attempts
- **GIVEN** listening is on
- **AND** the command spotting service stays away for a minute
- **WHEN** the service answers again
- **THEN** listening is on again within 5 seconds

## ADDED Requirements

### Requirement: The overlay shows that voice commands do not work while the service stays away
When the command spotting service has been away for 5 seconds while listening, the system SHALL show on the overlay that voice commands do not work now, until the service is back: alone in standby, beside the recording or processing sign during a dictation. The sign SHALL NOT take focus. Once the service is back, the overlay SHALL show only what the current state shows.

#### Scenario: A short outage shows nothing
- **GIVEN** listening is on and no dictation runs
- **WHEN** the command spotting service goes away and is back within 5 seconds
- **THEN** the overlay never shows that voice commands do not work
- **AND** no sound plays

#### Scenario: A long outage in standby shows the sign until the service is back
- **GIVEN** listening is on, no dictation runs and the overlay is hidden
- **WHEN** the command spotting service stays away for longer than 5 seconds
- **THEN** the overlay appears and shows that voice commands do not work
- **AND** once the service is back, the overlay hides

#### Scenario: A long outage during a dictation shows the sign beside the recording sign
- **GIVEN** a dictation runs and the overlay shows the recording sign
- **WHEN** the command spotting service stays away for longer than 5 seconds
- **THEN** the overlay shows that voice commands do not work beside the recording sign
- **AND** once the service is back, the overlay shows the recording sign alone

#### Scenario: A dictation that ends during a long outage keeps the sign
- **GIVEN** a dictation runs and the overlay shows that voice commands do not work
- **WHEN** the user ends the dictation with the hotkey and its text is pasted
- **THEN** the overlay still shows that voice commands do not work
- **AND** once the service is back, the overlay hides
