## ADDED Requirements

### Requirement: The language choice offers Russian, English and automatic detection
The spoken language control on the Dictation tab SHALL offer exactly Russian, English and automatic detection, and SHALL show Russian when the configuration file names no spoken language. A language the file names outside these three SHALL be shown as an extra entry under its own code and kept unless the user picks another one.

#### Scenario: The language choice offers three entries
- **WHEN** the user opens the spoken language choice
- **THEN** it offers Russian, English and automatic detection, and nothing else

#### Scenario: Russian is shown when the file names no language
- **GIVEN** the configuration file names no spoken language
- **WHEN** the user opens the Dictation tab
- **THEN** the spoken language control shows Russian

#### Scenario: Another language from the file is kept
- **GIVEN** the configuration file names German as the spoken language
- **WHEN** the user opens the Dictation tab and saves without touching the spoken language
- **THEN** the spoken language control shows the German code
- **AND** the configuration file still names German
