# file-transcription Specification

## Purpose

Transcribes a pre-recorded WAV file and prints the text to standard output, so the speech pipeline can be scripted and tested without a hotkey or a microphone.

## Requirements

### Requirement: A recording file runs through the whole pipeline
The system SHALL load its models, transcribe a 16 kHz mono 16-bit WAV file, refine the transcript unless refinement is turned off, and print the result to standard output. Startup misconfiguration SHALL be the only reason for exit code 2.

#### Scenario: Silence is processed without a startup failure
- **GIVEN** a recording of silence lasting 1, 10 or 60 seconds
- **WHEN** the user transcribes the recording file, with or without refinement
- **THEN** the app does not exit with a startup failure

### Requirement: Transcription meets the accuracy bar
The system SHALL transcribe the benchmark corpus (Russian and English speech) with a mean word error rate below 30 % after refinement.

#### Scenario: Benchmark corpus is transcribed within the accuracy bar
- **GIVEN** the benchmark corpus of Russian and English recordings with reference transcripts
- **WHEN** the user transcribes every recording of the corpus with refinement
- **THEN** the mean word error rate is below 30 %
