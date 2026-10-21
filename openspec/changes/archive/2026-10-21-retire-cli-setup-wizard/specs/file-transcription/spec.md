## ADDED Requirements

### Requirement: The command line is for developers and file mode
The command line SHALL offer only the options developers and tests need — the configuration file, file mode and its output, refinement on or off, the hotkey, voice control, the service addresses, the archive folder or none, the headless models download and its folder, the version and the help — and the help text of every option that changes a setting SHALL say it overrides the configuration for this run.

#### Scenario: The help lists the developer options
- **WHEN** the user asks the app for its help
- **THEN** the help lists exactly the developer options
- **AND** every option that changes a setting says it overrides the configuration for this run

### Requirement: Exit codes are stable
The process SHALL end with exit code 2 for a command line it does not accept or a configuration file it cannot read; in file mode with 1 when the recording cannot be read or transcribed, before any service is asked and with 2 when its service does not answer; with the headless models download with 0 when every file is present and 1 otherwise; and with 0 at Quit. In hotkey mode with the services the app manages, a missing permission, model or service SHALL never end the process.

#### Scenario: An unknown option exits with 2
- **WHEN** the user starts the app with an option it does not know
- **THEN** the process ends with exit code 2 and names the option

#### Scenario: File mode without its service exits with 2
- **GIVEN** no speech recognition service answers at the configured address
- **WHEN** the user transcribes a recording file
- **THEN** the process ends with exit code 2

#### Scenario: File mode with an unreadable recording exits with 1
- **GIVEN** a recording file that is not a WAV file
- **WHEN** the user transcribes it
- **THEN** the process ends with exit code 1
