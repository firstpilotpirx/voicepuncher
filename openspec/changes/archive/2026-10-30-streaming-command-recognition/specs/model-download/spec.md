## ADDED Requirements

### Requirement: Voice control downloads the models of its recognizer
The catalog SHALL hold the English and the Russian streaming models, each a pinned archive extracted into the models folder like the keyword spotting archive. With voice control on, the models needed SHALL be those of the chosen command recognizer: for the streaming recognizer the two streaming models and the voice gate model, and neither the Whisper small model nor the keyword spotting model; for the keyword spotter those two and the voice gate model.

#### Scenario: The streaming recognizer needs the streaming models
- **GIVEN** voice control is on with the streaming recognizer and no voice control model is on disk
- **WHEN** the app checks the models
- **THEN** the English and the Russian streaming models and the voice gate model are listed as Missing
- **AND** neither the Whisper small model nor the keyword spotting model is listed

#### Scenario: The keyword spotter needs its own models
- **GIVEN** voice control is on with the keyword spotter and no voice control model is on disk
- **WHEN** the app checks the models
- **THEN** the Whisper small model, the keyword spotting model and the voice gate model are listed as Missing
- **AND** no streaming model is listed
