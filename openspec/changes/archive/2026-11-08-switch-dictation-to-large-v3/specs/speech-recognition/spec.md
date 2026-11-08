## REMOVED Requirements

### Requirement: Dictation is recognized with the large-v3-turbo model
**Reason**: dictation moves to the full large-v3 model, which mishears the user's fast Russian speech with English terms less often.
**Migration**: the requirement "Dictation is recognized with the large-v3 model" replaces it; the turbo model stays usable by naming it.

## ADDED Requirements

### Requirement: Dictation is recognized with the large-v3 model
The service scripts and the app SHALL start the speech recognition service for dictation with the full Whisper large-v3 model, 16-bit, searching five candidate transcriptions per window, unless the user names another model when starting it, so that fast or unclear Russian speech with English terms is misheard less often than with large-v3-turbo.

#### Scenario: Dictation is recognized with the large-v3 model by default
- **GIVEN** the models that the service scripts download are on disk
- **WHEN** the user starts the services with the service scripts without naming a model
- **THEN** the speech recognition service for dictation recognizes speech with the full large-v3 model

#### Scenario: Dictation is recognized with the model the user names
- **GIVEN** the models that the service scripts download are on disk
- **WHEN** the user starts the services with the service scripts and names another speech recognition model for dictation
- **THEN** the speech recognition service for dictation recognizes speech with the named model
