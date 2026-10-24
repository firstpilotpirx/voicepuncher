## ADDED Requirements

### Requirement: Holding the hotkey records and releasing it pastes
In Hold mode, pressing the hotkey SHALL start a recording, and releasing it SHALL end the recording, transcribe it and paste the text into the focused app; a press shorter than the shortest recording the app keeps SHALL record nothing and paste nothing.

#### Scenario: A held hotkey records and a release pastes
- **GIVEN** the app is ready in Hold mode and a text field has keyboard focus
- **WHEN** the user holds the hotkey, speaks a phrase and releases the hotkey
- **THEN** a recording runs while the hotkey is held
- **AND** the phrase is transcribed and pasted into the text field after the release

#### Scenario: A short press records nothing
- **GIVEN** the app is ready in Hold mode
- **WHEN** the user releases the hotkey before the shortest recording the app keeps
- **THEN** nothing is transcribed or pasted
