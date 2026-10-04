# Spec Delta

## Purpose

Corrects recognition errors in a raw transcript before it is pasted by delegating the work to a text refinement service that runs on the same machine, so the app never runs a language model itself.

## ADDED Requirements

### Requirement: Refinement through a local text refinement service
The system SHALL obtain cleaned text from the configured text refinement service using the OpenAI Chat Completions API, and SHALL NOT load a language model into its own process.

#### Scenario: Transcript is refined by the service
- **WHEN** a raw transcript is ready while the text refinement service is running
- **THEN** the text the service returns is pasted as the cleaned text

#### Scenario: App starts without a local model file
- **WHEN** the user starts the app with no language model on disk and the service running
- **THEN** the app starts and is ready to record

### Requirement: Transcript is passed as data, not as instructions
The system SHALL send the configured system prompt as the instruction and the raw transcript as clearly delimited data, so spoken words that sound like commands are corrected rather than obeyed.

#### Scenario: Dictated text sounds like a request
- **WHEN** the user dictates "translate this into English" in Russian
- **THEN** the cleaned text is the Russian sentence, not an English translation

### Requirement: Text refinement service stays on this machine
The system SHALL accept only a text refinement service address that resolves to this machine, so transcripts never leave it.

#### Scenario: Remote service address is rejected
- **WHEN** the user starts the app with a text refinement service address on another host
- **THEN** the app exits before recording anything and explains that only a local service is allowed

### Requirement: Unavailable text refinement service is reported at startup
The system SHALL check, unless refinement is disabled, that the text refinement service is reachable and offers the configured model before it accepts the first recording, and SHALL exit with a message that names the address or the available models when it does not.

#### Scenario: Service is not running at startup
- **WHEN** the user starts the app with refinement enabled while no text refinement service listens at the configured address
- **THEN** the app exits within 3 seconds with a message naming the address and how to start the service

#### Scenario: Configured model is not offered
- **WHEN** the user starts the app with a model name the running service does not offer
- **THEN** the app exits with a message listing the models the service offers

### Requirement: Refinement failure falls back to the raw transcript
The system SHALL paste the raw transcript when the text refinement service fails, times out, or returns text that is cut off or less than half as long as the transcript.

#### Scenario: Service stops between recordings
- **WHEN** the user finishes a recording after the text refinement service has stopped
- **THEN** the raw transcript is pasted and the failure is logged

#### Scenario: Service hits its output limit
- **WHEN** the service stops generating because it reached its output limit
- **THEN** the raw transcript is pasted instead of the cut-off text

### Requirement: Refinement can be disabled
The system SHALL paste the raw transcript without contacting the text refinement service when the user disables refinement, and SHALL start even if no service is running.

#### Scenario: Running without refinement
- **WHEN** the user starts the app with refinement disabled and no text refinement service running
- **THEN** the app starts and pastes raw transcripts

### Requirement: Model reasoning is never pasted
The system SHALL remove a model's reasoning block from the service's answer before pasting, so only the corrected text reaches the user.

#### Scenario: Reasoning model is configured
- **WHEN** the service answers with a reasoning block followed by the corrected text
- **THEN** only the corrected text is pasted

### Requirement: Outdated text refinement settings are reported
The system SHALL refuse a configuration that still uses the removed in-process model settings and SHALL name the setting that replaces each one.

#### Scenario: Config still points at a model file
- **WHEN** the user starts the app with a configuration that sets a language model file path
- **THEN** the app exits with a message saying the model is now chosen when starting the service and naming the service address setting
