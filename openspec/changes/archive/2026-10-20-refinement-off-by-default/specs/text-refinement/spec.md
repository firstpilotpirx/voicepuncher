## ADDED Requirements

### Requirement: Refinement is off unless the user turns it on
The system SHALL paste the raw transcript without contacting the text refinement service, and SHALL start even if no service is running, unless the user turns refinement on on the command line or in the configuration; turning refinement off on the command line SHALL win over the configuration.

#### Scenario: Default settings paste the raw transcript
- **GIVEN** no text refinement service is listening at the configured address
- **WHEN** the user transcribes a recording of speech with default settings
- **THEN** the raw transcript is pasted

#### Scenario: Refinement turned on in the configuration
- **GIVEN** the configuration turns refinement on
- **AND** no text refinement service is listening at the configured address
- **WHEN** the user starts the app
- **THEN** the app exits with a message naming the text refinement service address and how to start it

#### Scenario: Turning refinement off on the command line wins over the configuration
- **GIVEN** the configuration turns refinement on
- **AND** no text refinement service is listening at the configured address
- **WHEN** the user transcribes a recording of speech with refinement turned off on the command line
- **THEN** the raw transcript is pasted

### Requirement: Running the app with one command starts the text refinement service only when refinement is on
The one command that runs the app for real use SHALL start neither the text refinement service nor need its model on disk unless the user turns refinement on with that command, and SHALL start and use the service when the user does.

#### Scenario: One command without refinement
- **GIVEN** the text refinement model is not on disk
- **WHEN** the user runs the app with one command and default settings
- **THEN** the app starts, no text refinement service is started, and a dictation is pasted as its raw transcript

#### Scenario: One command with refinement turned on
- **WHEN** the user runs the app with one command and turns refinement on
- **THEN** the text refinement service is started and a dictation is pasted as refined text

## MODIFIED Requirements

### Requirement: Unavailable text refinement service is reported at startup
The system SHALL check, when refinement is turned on, that the text refinement service is reachable and offers the configured model before it accepts the first recording, and SHALL exit with a message that names the address or the available models when it does not.

#### Scenario: Service is not running at startup
- **WHEN** the user starts the app with refinement turned on while no text refinement service listens at the configured address
- **THEN** the app exits within 3 seconds with a message naming the address and how to start the service

#### Scenario: Configured model is not offered
- **WHEN** the user starts the app with refinement turned on and a model name the running service does not offer
- **THEN** the app exits with a message listing the models the service offers

## REMOVED Requirements

### Requirement: Refinement can be disabled
**Reason**: Refinement is now off by default; turning it off is covered by "Refinement is off unless the user turns it on".
**Migration**: Nothing to do: the option that turned refinement off is still accepted and keeps refinement off; the option that turns it on is new.
