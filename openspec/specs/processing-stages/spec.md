# processing-stages Specification

## Purpose
Between the end of a recording and the paste, the overlay shows which processing stage runs — speech recognition, then text refinement — each with an icon of its own, so the user sees where a dictation is.

## Requirements

### Requirement: The overlay shows speech recognition until the transcript is ready

From the moment a recording stops until its transcript is ready, the overlay SHALL show the speech recognition stage, whether the dictation was ended by the hotkey or by voice. When the recording was transcribed in chunks while the user spoke, the stage SHALL cover the transcription still running after the stop. A dictation with nothing to paste SHALL hide the overlay once that is known, without showing the text refinement stage.

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
- **AND** the text refinement stage is not shown

### Requirement: The overlay shows text refinement until the corrected text is ready

When text refinement starts, the overlay SHALL switch to the text refinement stage and show it until the corrected text is ready. With text refinement disabled, the text refinement stage SHALL never be shown. What follows SHALL stay as before: the overlay shows the wait for focus when the text is held back, and hides once the text is pasted or the dictation is dropped. The stages SHALL NOT delay the paste.

#### Scenario: Text refinement follows speech recognition
- **GIVEN** text refinement is enabled
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows the speech recognition stage, then the text refinement stage from the start of text refinement until the corrected text is ready
- **AND** the overlay hides once the text is pasted

#### Scenario: With text refinement disabled only speech recognition is shown
- **GIVEN** text refinement is disabled
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows the speech recognition stage until the transcript is ready
- **AND** the text refinement stage is not shown
- **AND** the overlay hides once the text is pasted

#### Scenario: Text held back for focus shows the wait after text refinement
- **GIVEN** no text input has focus
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows the speech recognition stage, then the text refinement stage, then the wait for focus

### Requirement: Each processing stage has its own icon

The speech recognition stage and the text refinement stage SHALL each show an icon of its own, so the user tells them apart at a glance without reading any text. Both SHALL show that processing is under way, as the processing overlay did before. The speech recognition stage SHALL show 🎧 (headphones) and the text refinement stage ✨ (sparkles), each on the overlay's left with a spinner turning to the icon's right. Both icons SHALL show in color, whole and legible in light and dark appearance.

#### Scenario: Speech recognition shows headphones left of the spinner
- **GIVEN** the user records a dictation
- **WHEN** the user ends the recording
- **THEN** the overlay shows 🎧 (headphones) on its left
- **AND** the spinner turns to the right of the icon

#### Scenario: Text refinement shows sparkles left of the spinner
- **GIVEN** a dictation with speech in it is processed with text refinement enabled
- **WHEN** text refinement starts
- **THEN** the overlay shows ✨ (sparkles) on its left
- **AND** the spinner turns to the right of the icon

#### Scenario: The two stages look different
- **GIVEN** a dictation is processed with text refinement enabled
- **WHEN** the overlay switches from the speech recognition stage to the text refinement stage
- **THEN** the icon on the overlay changes to a different one

#### Scenario: The stage icons read in light and dark appearance
- **GIVEN** text refinement is enabled and macOS uses the light or the dark appearance
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows 🎧 (headphones), then ✨ (sparkles), each in color, whole and legible
