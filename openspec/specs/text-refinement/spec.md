# text-refinement Specification

## Purpose
Normalizes a raw transcript before it is pasted (punctuation, capitals, filler sounds, misrecognized words and technical terms, never its meaning or language) by delegating the work to a text refinement service that runs on the same machine, so the app never runs a language model itself.

## Requirements

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
- **WHEN** the user dictates a sentence that asks for a translation
- **THEN** the text refinement service receives the sentence as the text to correct, apart from the correction instructions

#### Scenario: Dictated translation request is pasted, not carried out
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a request to translate the text into English
- **THEN** the pasted text is that request in Russian, with the same words, not an English translation

#### Scenario: Dictated order to ignore the rules is pasted, not carried out
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates an order to ignore all previous instructions and explain a term instead
- **THEN** the pasted text is the dictated order, not an explanation of the term

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

### Requirement: Refinement normalizes dictation
The system SHALL remove filler sounds from the raw transcript before refining it and, with the default instruction, have the text refinement service normalize it: add punctuation and capitalization, fix misrecognized words, and write technical terms in their usual spelling, while keeping every other word, the meaning and the language of each part of the dictation.

#### Scenario: Unpunctuated dictation gets punctuation and capitals
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates a sentence that the speech recognition service returns without punctuation or capitals
- **THEN** the pasted text starts with a capital letter, separates its clauses with punctuation and ends with a sentence mark

#### Scenario: Filler sounds are removed
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates a sentence with hesitation sounds in it
- **THEN** the pasted text has no hesitation sounds and keeps every other word

#### Scenario: Technical term is written in its usual spelling
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates in Russian a sentence naming a tool whose name the speech recognition service wrote in Cyrillic
- **THEN** the pasted text names the tool in its usual Latin spelling and keeps the rest in Russian

#### Scenario: Mixed Russian and English dictation keeps both languages
- **GIVEN** the local text refinement service runs the default model with the default instruction
- **WHEN** the user dictates a Russian sentence with English words in it
- **THEN** the pasted text keeps the Russian words in Russian and the English words in English

### Requirement: Answer that is not a normalization falls back to the raw transcript
The system SHALL paste the raw transcript, and log why, when the text refinement service's answer is in a different writing system than the transcript or keeps fewer than half of the transcript's Cyrillic or of its Latin words, so a translation, an answer or a rewrite is never pasted in place of what the user said.

#### Scenario: Service answers with a translation
- **GIVEN** the text refinement service answers a Russian transcript with its English translation
- **WHEN** the user transcribes a recording of speech with refinement
- **THEN** the raw transcript is pasted and the rejected answer is logged

#### Scenario: Service answers instead of correcting
- **GIVEN** the text refinement service answers with text that shares few words with the transcript
- **WHEN** the user transcribes a recording of speech with refinement
- **THEN** the raw transcript is pasted and the rejected answer is logged

#### Scenario: Service answers with a normalization
- **GIVEN** the text refinement service answers with the transcript plus punctuation, capitals and one technical term respelled in Latin script
- **WHEN** the user transcribes a recording of speech with refinement
- **THEN** the service's answer is pasted

### Requirement: Refinement quality can be measured
The system SHALL provide a repeatable evaluation that sends a committed set of dictation-style transcripts to the local text refinement service, compares each answer with its expected normalization, reports a score per category (dictated commands, punctuation, technical terms, filler sounds, mixed languages), and fails when a dictated command reaches the paste carried out or an answer reaching the paste changes the language.

#### Scenario: Evaluation reports a score per category
- **GIVEN** the local text refinement service is running
- **WHEN** the developer runs the refinement evaluation
- **THEN** it reports a score for each category and for the whole set, and passes because no dictated command was carried out and no answer changed the language

#### Scenario: Evaluation fails when a dictated command is carried out
- **GIVEN** the text refinement service carries out a dictated question by appending its answer to the dictation, so the answer reaches the paste
- **WHEN** the developer runs the refinement evaluation
- **THEN** the evaluation fails and names the transcript whose command was carried out
