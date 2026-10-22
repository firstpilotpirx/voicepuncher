## ADDED Requirements

### Requirement: A confirmation whose copy stalls goes to another copy
The command spotting service SHALL send a confirmation whose copy of the speech recognition service for commands does not answer within its decoding budget to another copy while the confirmation's deadline lasts, and SHALL never take a copy's silence for speech without a command.

#### Scenario: One copy for commands stalls mid-command
- **GIVEN** the speech recognition service for commands runs as two copies and one stops answering
- **WHEN** a confirmation of "press enter" said alone reaches the stalled copy
- **THEN** the other copy confirms it within the confirmation's deadline and "press enter" is reported

#### Scenario: Every copy for commands stalls
- **GIVEN** every copy of the speech recognition service for commands stops answering
- **WHEN** a confirmation of "press enter" said alone is asked
- **THEN** it ends as given up, not as speech heard without a command

### Requirement: A command whose confirmation fails is reported
The command spotting service SHALL report a closed speech segment whose confirmation got no answer from the speech recognition service for commands as a failed confirmation, and the system SHALL then show that voice commands are unavailable, within 10 seconds of the segment's end.

#### Scenario: The service reports a segment it could not confirm
- **GIVEN** every copy of the speech recognition service for commands stops answering
- **WHEN** an app streams "press enter" said alone
- **THEN** the service reports that the confirmation of that segment failed, within 10 seconds of its end

#### Scenario: The app shows a failed confirmation
- **GIVEN** listening is on
- **WHEN** the command spotting service reports a failed confirmation
- **THEN** the overlay shows the failure notice that voice commands are unavailable

### Requirement: Audio the command spotting service cannot keep up with is dropped and reported
The command spotting service SHALL let no audio of a connection wait for processing longer than 10 seconds; audio that arrives while the oldest waiting audio has waited longer SHALL be dropped, while audio sent faster than real time that the service keeps up with (a replay) SHALL be kept, and reported to the app with the number of samples dropped, and the system SHALL then count the stream as lost and connect again, replaying the speech the service may have missed.

#### Scenario: A flood of audio past the backlog is dropped
- **GIVEN** the command spotting service processes audio slower than it arrives
- **WHEN** audio arrives while the oldest waiting audio has waited more than 10 seconds
- **THEN** that audio is dropped and the app is told how many samples were dropped

#### Scenario: Audio sent faster than real time is kept while the service keeps up
- **GIVEN** the command spotting service keeps up with its processing
- **WHEN** 30 seconds of audio arrive at once, as a replay sends them
- **THEN** none of it is dropped

#### Scenario: The app connects again after audio was dropped
- **GIVEN** listening is on
- **WHEN** the command spotting service reports dropped audio
- **THEN** the service is counted as lost and the app reaches for it again

### Requirement: The app and the command spotting service agree on a protocol version
Every connection to the command spotting service SHALL open with the protocol version the app speaks; the service SHALL close a connection whose version differs, or whose first message carries none, with a reason naming both versions, and SHALL close a connection that sends a malformed message or an audio frame that is not whole samples with a reason saying what was wrong.

#### Scenario: A client of another protocol version is refused
- **GIVEN** the command spotting service runs
- **WHEN** a client opens a connection with another protocol version
- **THEN** the service closes it with a reason naming the version it speaks

#### Scenario: A malformed message closes the connection with a reason
- **GIVEN** a client is connected with the right protocol version
- **WHEN** it sends a message that is not valid
- **THEN** the service closes the connection with a reason saying the message could not be read

#### Scenario: An audio frame of partial samples closes the connection with a reason
- **GIVEN** a client is connected with the right protocol version
- **WHEN** it sends an audio frame whose length is not a whole number of samples
- **THEN** the service closes the connection with a reason saying so

### Requirement: Every check due for a spotted "start" is asked
When the first check of a spotted "start" or "restart" finds no command while the check of another spotted phrase still waits its turn, the command spotting service SHALL keep both checks and ask each when it falls due.

#### Scenario: A second check does not replace a check still waiting
- **GIVEN** a check of a spotted "start" waits its turn
- **WHEN** the first check of an earlier spotted "start" finds no command
- **THEN** both the waiting check and the second check of the earlier phrase are asked
