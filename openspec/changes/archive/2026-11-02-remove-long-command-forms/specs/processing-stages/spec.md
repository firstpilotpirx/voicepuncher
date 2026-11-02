## MODIFIED Requirements

### Requirement: The overlay shows speech recognition until the transcript is ready

From the moment a recording stops until its transcript is ready, the overlay SHALL show the speech recognition stage, whether the dictation was ended by the hotkey or by voice. When the recording was transcribed in chunks while the user spoke, the stage SHALL cover the transcription still running after the stop. A dictation with nothing to paste SHALL hide the overlay once that is known.

#### Scenario: A hotkey dictation shows speech recognition after the stop
- **GIVEN** the user records a dictation with the hotkey
- **WHEN** the user ends the recording with the hotkey
- **THEN** the overlay shows the speech recognition stage from the stop until the transcript is ready

#### Scenario: A dictation ended by voice shows speech recognition after the stop
- **GIVEN** voice control is turned on and a dictation runs
- **WHEN** the user says "stop"
- **THEN** the overlay shows the speech recognition stage from the stop until the transcript is ready

#### Scenario: A dictation transcribed while the user speaks shows speech recognition for its last part
- **GIVEN** the user records a dictation long enough to be transcribed in parts while the user speaks
- **WHEN** the user ends the recording
- **THEN** the overlay shows the speech recognition stage from the stop until the last part is transcribed

#### Scenario: A dictation without speech hides the overlay after speech recognition
- **GIVEN** the user records a dictation with no speech in it
- **WHEN** the user ends the recording
- **THEN** the overlay shows the speech recognition stage, then hides
