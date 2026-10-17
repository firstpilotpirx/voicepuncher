## ADDED Requirements

### Requirement: Dictation is recognized with the large-v3-turbo model
The service scripts SHALL start the speech recognition service for dictation — the one the app sends dictations and recording files to — with the Whisper large-v3-turbo model, 5-bit quantized, unless the user names another model when starting it, so that long speech under difficult recording conditions misses at most one spoken word in ten.

#### Scenario: Dictation is recognized with the large-v3-turbo model by default
- **GIVEN** the models that the service scripts download are on disk
- **WHEN** the user starts the services with the service scripts without naming a model
- **THEN** the speech recognition service for dictation recognizes speech with the large-v3-turbo model

#### Scenario: Dictation is recognized with the model the user names
- **GIVEN** the models that the service scripts download are on disk
- **WHEN** the user starts the services with the service scripts and names another speech recognition model for dictation
- **THEN** the speech recognition service for dictation recognizes speech with the named model
- **AND** the speech recognition service for commands keeps the small model

## MODIFIED Requirements

### Requirement: Audio without speech never stops the speech recognition service
The speech recognition service SHALL answer a request whose audio holds no speech with an empty transcript and SHALL keep answering later requests, in every answer format the system asks for, including the detailed format the command spotting service uses to confirm commands.

#### Scenario: Command confirmation of an utterance without speech
- **GIVEN** the service scripts started the speech recognition service for commands
- **WHEN** the command spotting service asks it to confirm an utterance that holds no speech
- **THEN** no words are heard and the service answers the next request
