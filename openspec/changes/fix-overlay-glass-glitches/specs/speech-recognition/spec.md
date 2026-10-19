## MODIFIED Requirements

### Requirement: Spoken language follows the configuration
The system SHALL send the configured spoken language with every request; the value `auto` SHALL make the service detect the language and still return a transcript. With `auto`, the transcript SHALL be only in the user's configured languages (Russian and English by default): a request the service detects as another language SHALL be sent again with the first of them fixed, and that transcript used.

#### Scenario: Automatic language detection
- **WHEN** the configured language is `auto` and the user records Russian speech
- **THEN** the raw transcript contains the Russian words that were spoken

#### Scenario: Fixed language
- **WHEN** the configured language is `en` and the user records English speech
- **THEN** the raw transcript contains the English words that were spoken

#### Scenario: Short Russian speech is never transcribed as Korean
- **GIVEN** the configured language is `auto` and the user's languages are Russian and English
- **AND** the service detects a short Russian phrase as Korean
- **WHEN** the user records that phrase
- **THEN** the raw transcript is the phrase in Russian, with no Korean characters

#### Scenario: English speech stays English with automatic detection
- **GIVEN** the configured language is `auto` and the user's languages are Russian and English
- **WHEN** the user records English speech
- **THEN** the raw transcript contains the English words that were spoken, in Latin letters
