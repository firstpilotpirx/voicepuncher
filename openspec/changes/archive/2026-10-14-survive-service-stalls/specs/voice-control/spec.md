## REMOVED Requirements

### Requirement: Voice control needs the command spotting service
**Reason**: It turned listening off for good when the command spotting service stopped answering, so one slow confirmation ended voice control until the app was restarted (incident of 2026-10-14).
**Migration**: Split into "Voice control needs the command spotting service to start" (the startup check, unchanged) and "The app keeps reaching for the command spotting service while it is away" (reconnect instead of giving up).

## MODIFIED Requirements

### Requirement: Listening stays on while voice control is on
With voice control on, the system SHALL start listening as soon as the app starts, without a hotkey press, and SHALL keep listening until the app exits; while the command spotting service is away, listening SHALL resume on its own once the service answers again. The hotkey SHALL NOT turn listening off. Turning listening on SHALL play its sound.

#### Scenario: Listening is on at start
- **GIVEN** voice control is turned on
- **WHEN** the app starts
- **THEN** listening is on
- **AND** the listening-on sound plays

#### Scenario: Listening stays on after a hotkey dictation
- **GIVEN** listening is on
- **WHEN** the user records a dictation with the hotkey
- **THEN** listening is on
- **AND** saying "press enter" presses Return

## ADDED Requirements

### Requirement: Voice control needs the command spotting service to start
With voice control on, the system SHALL refuse to start, with a message naming the command spotting service and how to start it, when that service does not answer.

#### Scenario: The app refuses to start without the command spotting service
- **GIVEN** voice control is turned on
- **AND** the command spotting service is not running
- **WHEN** the app starts
- **THEN** the app exits with an error that names the command spotting service and how to start it

### Requirement: The app keeps reaching for the command spotting service while it is away
When the command spotting service stops answering while listening, the system SHALL log an error naming the service, play the listening-off sound and keep trying to reach the service, with a growing pause of at most 4 seconds, until it answers or the app exits; it SHALL log the first failed attempt and every tenth, so a long outage does not flood the log.

#### Scenario: The app keeps reaching for the command spotting service when it goes away
- **GIVEN** listening is on
- **WHEN** the command spotting service stops answering
- **THEN** an error naming the command spotting service is logged
- **AND** the listening-off sound plays
- **AND** the app tries to reach the service again, logging its first failed attempt and every tenth

### Requirement: Listening comes back when the command spotting service is back
When the command spotting service answers again after it went away, the system SHALL resume listening for the commands of the current state within 5 seconds, play the listening-on sound and log that listening is back, without a restart of the app.

#### Scenario: Commands work again after the command spotting service restarts
- **GIVEN** listening is on and no dictation runs
- **AND** the command spotting service went away and was started again
- **WHEN** the user says "press enter" 5 seconds after the service answers again
- **THEN** the Return key is pressed in the focused app
- **AND** the listening-on sound played when the service came back

#### Scenario: Listening is not given up after many failed attempts
- **GIVEN** listening is on
- **AND** the command spotting service stays away for a minute
- **WHEN** the service answers again
- **THEN** listening is on again within 5 seconds

### Requirement: A slow command spotting service does not stop listening
The system SHALL keep listening while the command spotting service answers late, however long its spotting results take, as long as the service still answers on its connection; only a dropped connection or a service that answers nothing at all for 5 seconds SHALL count as the service going away.

#### Scenario: A confirmation that takes longer than 10 seconds keeps listening on
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service takes 15 seconds to answer one confirmation of the command spotting service
- **WHEN** the user keeps dictating through those 15 seconds and then says "stop dictation"
- **THEN** listening is still on and no error about the command spotting service is logged
- **AND** the dictation ends and its text is pasted

### Requirement: A dictation keeps recording while the command spotting service is away
The system SHALL keep a running dictation recording when the command spotting service goes away; the hotkey SHALL still end it, and once the service is back the dictation's voice commands SHALL end it as usual.

#### Scenario: The hotkey ends a dictation while the service is away
- **GIVEN** a dictation started by voice runs
- **AND** the command spotting service went away during it
- **WHEN** the user presses the hotkey to stop
- **THEN** the dictation ends and its text is pasted

#### Scenario: "stop dictation" ends a dictation after the service is back
- **GIVEN** a dictation started by voice runs
- **AND** the command spotting service went away during it and is back
- **WHEN** the user says "stop dictation"
- **THEN** the dictation ends and its text is pasted

### Requirement: A command said while the command spotting service was away still counts
The system SHALL send the service the speech heard during the last 10 seconds of an outage once the service is back, so a command said in that time is carried out late rather than lost.

#### Scenario: "stop dictation" said during a short outage ends the dictation
- **GIVEN** a dictation runs
- **AND** the command spotting service is away for 3 seconds
- **WHEN** the user says "stop dictation" during those 3 seconds
- **THEN** the dictation ends once the service is back, and the words "stop dictation" are not in the pasted text
