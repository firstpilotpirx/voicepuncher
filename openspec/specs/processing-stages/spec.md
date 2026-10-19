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

When text refinement starts, the overlay SHALL switch to the text refinement stage and show it until the corrected text is ready. With text refinement disabled, the text refinement stage SHALL never be shown. What follows SHALL be: the overlay shows the wait for focus when the text is held back, the Pasted sign once the text is pasted, and hides once the dictation is dropped. The stages SHALL NOT delay the paste.

#### Scenario: Text refinement follows speech recognition
- **GIVEN** text refinement is enabled
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows the speech recognition stage, then the text refinement stage from the start of text refinement until the corrected text is ready
- **AND** the overlay shows the Pasted sign once the text is pasted, then hides

#### Scenario: With text refinement disabled only speech recognition is shown
- **GIVEN** text refinement is disabled
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows the speech recognition stage until the transcript is ready
- **AND** the text refinement stage is not shown
- **AND** the overlay shows the Pasted sign once the text is pasted, then hides

#### Scenario: Text held back for focus shows the wait after text refinement
- **GIVEN** no text input has focus
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows the speech recognition stage, then the text refinement stage, then the wait for focus

### Requirement: Each processing stage has its own icon and label

The speech recognition stage SHALL show a breathing ear and "Recognizing", the text refinement stage breathing sparkles and "Refining text", each icon on the overlay's left with a spoke spinner right of the label, so the user tells them apart at a glance. The spinner SHALL be indeterminate: it SHALL NOT fill up or suggest how long is left. Both icons SHALL be legible in light and dark appearance.

#### Scenario: Speech recognition shows an ear left of its label and spinner
- **GIVEN** the user records a dictation
- **WHEN** the user ends the recording
- **THEN** the overlay shows a breathing ear on its left, then "Recognizing"
- **AND** a spoke spinner turns to the right of the label

#### Scenario: Text refinement shows sparkles left of its label and spinner
- **GIVEN** a dictation with speech in it is processed with text refinement enabled
- **WHEN** text refinement starts
- **THEN** the overlay shows breathing sparkles on its left, then "Refining text"
- **AND** a spoke spinner turns to the right of the label

#### Scenario: The two stages look different
- **GIVEN** a dictation is processed with text refinement enabled
- **WHEN** the overlay switches from the speech recognition stage to the text refinement stage
- **THEN** the icon and the label on the overlay change to different ones

#### Scenario: The stage icons read in light and dark appearance
- **GIVEN** text refinement is enabled and macOS uses the light or the dark appearance
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay shows the ear, then the sparkles, each whole and legible

#### Scenario: A long speech recognition never looks nearly done
- **GIVEN** the user records a long dictation
- **WHEN** speech recognition runs for several seconds after the stop
- **THEN** the spinner keeps turning without filling up

### Requirement: Text refinement shows the raw transcript dimmed

While the text refinement stage shows, the text window under the overlay SHALL show the raw transcript being refined, dimmed, so the user sees what is being corrected. The speech recognition stage SHALL show no text window.

#### Scenario: Refinement shows the dimmed raw transcript
- **GIVEN** text refinement is enabled
- **WHEN** text refinement of a dictation with speech in it starts
- **THEN** the text window shows that dictation's raw transcript, dimmed
