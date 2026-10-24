# file-transcription Specification

## Purpose

Transcribes a pre-recorded WAV file and prints the text to standard output, so the speech pipeline can be scripted and tested without a hotkey or a microphone.

## Requirements

### Requirement: A recording file runs through the whole pipeline
The system SHALL load its models, transcribe a 16 kHz mono 16-bit WAV file and print the transcript to standard output. Startup misconfiguration SHALL be the only reason for exit code 2.

#### Scenario: Silence is processed without a startup failure
- **GIVEN** a recording of silence lasting 1, 10 or 60 seconds
- **WHEN** the user transcribes the recording file
- **THEN** the app does not exit with a startup failure

### Requirement: Transcription meets the accuracy bar
The system SHALL transcribe a fixed sample of the benchmark corpus, 100 recordings with Russian and English speech in equal shares, with a mean word error rate below 30 %.

#### Scenario: Benchmark corpus is transcribed within the accuracy bar
- **GIVEN** a fixed sample of 100 recordings of the benchmark corpus, half Russian and half English, with reference transcripts
- **WHEN** the user transcribes every recording of the sample
- **THEN** the mean word error rate is below 30 %

### Requirement: The command line is for developers and file mode
The command line SHALL offer only the options developers and tests need — the configuration file, file mode and its output, the hotkey, voice control, the service addresses, the archive folder or none, the headless models download and its folder, the version and the help — and the help text of every option that changes a setting SHALL say it overrides the configuration for this run. It SHALL offer no option about text refinement.

#### Scenario: The help lists the developer options
- **WHEN** the user asks the app for its help
- **THEN** the help lists exactly the developer options
- **AND** every option that changes a setting says it overrides the configuration for this run

#### Scenario: A text refinement option is refused
- **WHEN** the user starts the app with the former option that turned text refinement off
- **THEN** the process ends with exit code 2 and names the option

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
