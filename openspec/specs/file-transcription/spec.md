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
The system SHALL transcribe a fixed sample of the benchmark corpus, 10 or 100 recordings with Russian and English speech in equal shares, with a mean word error rate no worse than the accepted baseline of that sample size plus the margin. The baseline SHALL be kept in the repository with the commit it came from. The margin SHALL be 0.01 while the stage journal holds fewer than 10 measured runs of that sample size, and otherwise 1.5 times the standard deviation of the last 10, never below 0.01. A run better than the baseline by more than the margin SHALL pass and say how to accept the new baseline. A sample size with no baseline SHALL fail and say how to record one.

#### Scenario: Benchmark corpus is transcribed within the accuracy bar
- **GIVEN** a fixed sample of 100 recordings of the benchmark corpus, half Russian and half English, with reference transcripts
- **WHEN** the user transcribes every recording of the sample
- **THEN** the mean word error rate is no worse than the accepted baseline plus the margin

#### Scenario: A run worse than the baseline fails
- **GIVEN** an accepted baseline mean word error rate of 0.05 and a margin of 0.01
- **WHEN** a run measures 0.07
- **THEN** the run fails, naming the measured rate, the baseline, the margin and the commit of the baseline

#### Scenario: A run within the margin passes
- **GIVEN** an accepted baseline mean word error rate of 0.05 and a margin of 0.01
- **WHEN** a run measures 0.055
- **THEN** the run passes

#### Scenario: A clearly better run passes with a hint
- **GIVEN** an accepted baseline mean word error rate of 0.05 and a margin of 0.01
- **WHEN** a run measures 0.03
- **THEN** the run passes and says that `just accuracy-baseline` accepts the better rate

#### Scenario: The margin follows the measured noise
- **GIVEN** a stage journal with 10 measured runs of the sample size
- **WHEN** the margin is computed
- **THEN** it is 1.5 times the standard deviation of those 10 rates, or 0.01 when that is smaller

#### Scenario: A sample size without a baseline fails
- **GIVEN** a baseline file with no entry for the sample size of the run
- **WHEN** a run measures its mean word error rate
- **THEN** the run fails and says that `just accuracy-baseline` records one

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
