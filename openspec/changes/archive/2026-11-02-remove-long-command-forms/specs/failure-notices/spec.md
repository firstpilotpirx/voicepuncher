## MODIFIED Requirements

### Requirement: A lost dictation is announced
The system SHALL show a failure notice saying the dictation was lost, and why, when the transcript of a dictation cannot be produced, whether the dictation was started by the hotkey or by voice.

#### Scenario: A hotkey dictation is lost
- **GIVEN** the speech recognition service does not answer
- **WHEN** the user stops a dictation started with the hotkey
- **THEN** nothing is pasted and the overlay shows that the dictation was lost because speech recognition failed

#### Scenario: A voice dictation is lost
- **GIVEN** voice control is on and the speech recognition service does not answer
- **WHEN** the user says "stop" after a dictation started by voice
- **THEN** nothing is pasted and the overlay shows that the dictation was lost because speech recognition failed

### Requirement: A voice command that fails is announced
The system SHALL show a failure notice naming the voice command that could not be carried out, and why, when a recognized voice command fails; voice control keeps listening for the next command.

#### Scenario: A key press is refused
- **GIVEN** voice control is on and pressing keys in the focused app fails
- **WHEN** the user says "enter"
- **THEN** the overlay shows that "press enter" failed
- **AND** the next voice command is still carried out
