## MODIFIED Requirements

### Requirement: Spoken language follows the configuration
The system SHALL send the configured spoken language with every request; the value `auto` SHALL make the service detect the language and still return a transcript. When the configuration names no spoken language, the spoken language SHALL be Russian. With `auto`, the transcript SHALL be only in the user's configured languages (Russian and English by default): a request the service detects as another language SHALL be sent again with the first of them fixed, and that transcript used.

#### Scenario: Automatic language detection
- **WHEN** the configured language is `auto` and the user records Russian speech
- **THEN** the raw transcript contains the Russian words that were spoken

#### Scenario: Fixed language
- **WHEN** the configured language is `en` and the user records English speech
- **THEN** the raw transcript contains the English words that were spoken

#### Scenario: Russian is the language when none is configured
- **GIVEN** the configuration names no spoken language
- **WHEN** the user dictates
- **THEN** every request asks the service for Russian

#### Scenario: English speech stays English with the default language
- **GIVEN** the configuration names no spoken language
- **WHEN** the user records English speech
- **THEN** the raw transcript contains the English words that were spoken, in Latin letters

#### Scenario: Short Russian speech is never transcribed as Korean
- **GIVEN** the configured language is `auto` and the user's languages are Russian and English
- **AND** the service detects a short Russian phrase as Korean
- **WHEN** the user records that phrase
- **THEN** the raw transcript is the phrase in Russian, with no Korean characters

#### Scenario: English speech stays English with automatic detection
- **GIVEN** the configured language is `auto` and the user's languages are Russian and English
- **WHEN** the user records English speech
- **THEN** the raw transcript contains the English words that were spoken, in Latin letters

## ADDED Requirements

### Requirement: Stopping a dictation waits only for its last chunk
When the configuration names no chunk threshold, the chunk threshold SHALL be 10 seconds. Every chunk of a dictation but the last SHALL be cut and handed on for transcription while the user goes on dictating, so after the user stops only the last chunk, at most the chunk threshold long, is left to transcribe.

#### Scenario: A long dictation is transcribed while it goes on
- **GIVEN** the configuration names no chunk threshold
- **WHEN** the user dictates for 25 seconds and stops
- **THEN** every chunk but the last was cut and handed on for transcription before the stop
- **AND** the last chunk is at most 10 seconds long

#### Scenario: A short dictation is one chunk
- **GIVEN** the configuration names no chunk threshold
- **WHEN** the user dictates for 8 seconds and stops
- **THEN** the dictation is handed on for transcription as one chunk after the stop
