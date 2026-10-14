## ADDED Requirements

### Requirement: Audio without speech never stops the speech recognition service
The speech recognition service SHALL answer a request whose audio holds no speech with an empty transcript and SHALL keep answering later requests, in every answer format the system asks for, including the detailed format the command spotting service uses to confirm commands.

#### Scenario: Command confirmation of an utterance without speech
- **GIVEN** the service scripts started the speech recognition service
- **WHEN** the command spotting service asks it to confirm an utterance that holds no speech
- **THEN** no words are heard and the service answers the next request
