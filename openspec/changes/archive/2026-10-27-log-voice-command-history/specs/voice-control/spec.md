## ADDED Requirements

### Requirement: Every utterance with a command heard is kept in the command history
With the command history on, the command spotting service SHALL write one command history entry for every utterance in which the keyword spotter spotted a selected phrase or the service reported a command, holding the audio the speech recognition service for commands heard, every phrase spotted in it, the transcript, the commands reported, which of them the speech recognition service heard, and the outcome: agreed, corrected, rejected or missed. An utterance with neither SHALL leave no entry.

#### Scenario: A command both models heard is kept as agreed
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the user says "press enter" and the keyword spotter spots it
- **THEN** a new command history entry holds playable audio of the utterance, the spotter's "press enter", the transcript and the reported "press enter", and its outcome is agreed

#### Scenario: A command only the speech recognition service heard is kept as missed
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the user says a command the keyword spotter does not spot and the speech recognition service hears it
- **THEN** a new command history entry names no spotted phrase, holds the transcript and the reported command, and its outcome is missed

#### Scenario: A spotted phrase the speech recognition service does not hear is kept as rejected
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the keyword spotter spots a phrase in an utterance in which the speech recognition service hears no command
- **THEN** a new command history entry holds the spotted phrase and the transcript, no command the speech recognition service heard, and its outcome is rejected

#### Scenario: A spotted phrase heard as another command is kept as corrected
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the keyword spotter spots one phrase in an utterance in which the speech recognition service hears another command
- **THEN** a new command history entry holds the spotted phrase, the transcript and the reported command, and its outcome is corrected

#### Scenario: Ordinary speech leaves no entry
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the user says a sentence with no command in it and the keyword spotter spots nothing
- **THEN** the command history holds no new entry

### Requirement: The command history is on for the app's own services and off elsewhere
The app SHALL turn the command history on by default, into a folder under the app's home folder beside the dictation archive, for the copies of the command spotting service it starts itself, and SHALL let the user turn it off in the configuration. A command spotting service not given a command history folder SHALL write no entry, so the service scripts, tests and benchmarks never write into the user's command history.

#### Scenario: The app's own services keep the command history by default
- **GIVEN** the default configuration with voice control on
- **WHEN** the app starts the copies of the command spotting service
- **THEN** each copy is told to keep the command history in a folder under the app's home folder

#### Scenario: Command history turned off
- **GIVEN** the user turned the command history off and voice control is on
- **WHEN** the app starts the copies of the command spotting service
- **THEN** no copy is told to keep a command history

#### Scenario: A service given no folder writes no entry
- **GIVEN** a command spotting service started without a command history folder
- **WHEN** a command is said to it
- **THEN** it reports the command and writes no command history entry

### Requirement: The command history stays within its size limit
After writing an entry, the command spotting service SHALL remove the oldest command history entries until the command history fits its size limit, which is the dictation archive's size limit, never removing the entry just written.

#### Scenario: The oldest command history entries go first
- **GIVEN** a command history whose entries already fill its size limit
- **WHEN** a new entry is written
- **THEN** the oldest entries are removed until the command history fits, and the newest entries, the new one included, stay

### Requirement: Keeping the command history never costs a command
The command spotting service SHALL report every command as it does with the command history off, and a failure to write or prune the command history SHALL be logged as a warning without changing what is reported or stopping the service.

#### Scenario: The command history folder cannot be written
- **GIVEN** a command history folder the service cannot write to and voice control is listening
- **WHEN** the user says "press enter"
- **THEN** Return is pressed as usual and the service log warns that the utterance was not kept
