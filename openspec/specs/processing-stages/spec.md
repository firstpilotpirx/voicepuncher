# processing-stages Specification

## Purpose
Between the end of a recording and the paste, the overlay shows the speech recognition stage with an icon of its own, so the user sees where a dictation is; the transcript is pasted as soon as speech recognition ends.

## Requirements

### Requirement: The overlay shows speech recognition until the transcript is ready

From the moment a recording stops until its transcript is ready, the overlay SHALL show the speech recognition stage, whether the dictation was ended by the hotkey or by voice. When the recording was transcribed in chunks while the user spoke, the stage SHALL cover the transcription still running after the stop. A dictation with nothing to paste SHALL hide the overlay once that is known.

#### Scenario: A hotkey dictation shows speech recognition after the stop
- **GIVEN** the user records a dictation with the hotkey
- **WHEN** the user ends the recording with the hotkey
- **THEN** the overlay shows the speech recognition stage from the stop until the transcript is ready

#### Scenario: A dictation ended by voice shows speech recognition after the stop
- **GIVEN** voice control is turned on and a dictation runs
- **WHEN** the user says "stop dictation"
- **THEN** the overlay shows the speech recognition stage from the stop until the transcript is ready

#### Scenario: A dictation transcribed while the user speaks shows speech recognition for its last part
- **GIVEN** the user records a dictation long enough to be transcribed in parts while the user speaks
- **WHEN** the user ends the recording
- **THEN** the overlay shows the speech recognition stage from the stop until the last part is transcribed

#### Scenario: A dictation without speech hides the overlay after speech recognition
- **GIVEN** the user records a dictation with no speech in it
- **WHEN** the user ends the recording
- **THEN** the overlay shows the speech recognition stage, then hides

### Requirement: The transcript is pasted right after speech recognition

Once the transcript is ready, the overlay SHALL go straight from the speech recognition stage to what follows: the wait for focus when the text is held back, the Pasted sign once the text is pasted, and hidden once the dictation is dropped. No other processing stage SHALL be shown between speech recognition and the paste, and the stages SHALL NOT delay the paste.

#### Scenario: Speech recognition is followed by the paste
- **GIVEN** the user records a dictation with speech in it
- **WHEN** the user ends the recording
- **THEN** the overlay shows the speech recognition stage until the transcript is ready
- **AND** the overlay shows the Pasted sign once the text is pasted, then hides

#### Scenario: Text held back for focus shows the wait after speech recognition
- **GIVEN** no text input has focus
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows the speech recognition stage, then the wait for focus

### Requirement: Speech recognition shows an ear, its label and a spinner

The speech recognition stage SHALL show a breathing ear and "Recognizing" on the overlay's left with a spoke spinner right of the label. The spinner SHALL be indeterminate: it SHALL NOT fill up or suggest how long is left. The icon SHALL be legible in light and dark appearance.

#### Scenario: Speech recognition shows an ear left of its label and spinner
- **GIVEN** the user records a dictation
- **WHEN** the user ends the recording
- **THEN** the overlay shows a breathing ear on its left, then "Recognizing"
- **AND** a spoke spinner turns to the right of the label

#### Scenario: The stage icon reads in light and dark appearance
- **GIVEN** macOS uses the light or the dark appearance
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows the ear whole and legible

#### Scenario: A long speech recognition never looks nearly done
- **GIVEN** the user records a long dictation
- **WHEN** speech recognition runs for several seconds after the stop
- **THEN** the spinner keeps turning without filling up

### Requirement: Speech recognition shows no text window

While the speech recognition stage shows, the overlay SHALL show the stage in the pill alone, with no text window under it.

#### Scenario: Speech recognition shows no transcript
- **GIVEN** the user records a dictation with speech in it
- **WHEN** the user ends the recording
- **THEN** the overlay shows "Recognizing" in the pill and no text window under it
