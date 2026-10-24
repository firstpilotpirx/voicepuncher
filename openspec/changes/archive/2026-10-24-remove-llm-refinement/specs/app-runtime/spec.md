## MODIFIED Requirements

### Requirement: The pipeline starts from the configuration
The runtime SHALL start the dictation pipeline from a valid configuration once every service it needs answers, and SHALL report the state Ready with whether voice control is on; when a service does not answer, it SHALL report the state Failed naming that service and start no pipeline.

#### Scenario: Every service answers
- **GIVEN** a valid configuration with voice control off
- **AND** the speech recognition service answers
- **WHEN** the runtime starts
- **THEN** the dictation pipeline runs with that configuration
- **AND** the runtime state is Ready with voice control off

#### Scenario: A service does not answer
- **GIVEN** a valid configuration
- **AND** the speech recognition service does not answer
- **WHEN** the runtime starts
- **THEN** the runtime state is Failed with a reason naming the speech recognition service
- **AND** no dictation pipeline runs

### Requirement: A restart waits for the dictation in progress
When a reload arrives while the pipeline records, recognizes, waits for focus or pastes, the runtime SHALL report the state Restarting, let that dictation finish and paste, and only then restart the pipeline with the new configuration.

#### Scenario: Reload during a recording
- **GIVEN** the runtime is Ready
- **AND** a recording is in progress
- **WHEN** the runtime reloads a valid configuration
- **THEN** the runtime state is Restarting
- **AND** the recording keeps running until the hotkey stops it
- **AND** the dictation is pasted
- **AND** only then the pipeline restarts and the runtime state is Ready

#### Scenario: Reload while the pipeline waits for focus
- **GIVEN** the runtime is Ready
- **AND** a dictation waits for a text input to gain focus
- **WHEN** the runtime reloads a valid configuration
- **THEN** the runtime state is Restarting
- **AND** the pipeline restarts only after that dictation was pasted or given up
