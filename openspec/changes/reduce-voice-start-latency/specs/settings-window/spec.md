## ADDED Requirements

### Requirement: Voice control has a switch for the Whisper confirmation
The voice control settings SHALL hold a switch "Confirm commands with Whisper", on by default, with a line saying that off makes commands faster but lets the keyword spotter alone decide, and that the log and command history record whether Whisper agreed; saving it SHALL apply to the next command.

#### Scenario: The switch is on by default
- **GIVEN** the configuration file does not set the confirmation of commands
- **WHEN** the user opens the voice control settings
- **THEN** the switch "Confirm commands with Whisper" is on

#### Scenario: Switching it off applies to the next command
- **GIVEN** voice control is on
- **WHEN** the user switches "Confirm commands with Whisper" off and saves
- **THEN** the next "start start" starts a dictation without waiting for Whisper
