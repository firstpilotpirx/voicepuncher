## ADDED Requirements

### Requirement: In Claude, the hotkey drives Claude's own dictation
When the hotkey starts a recording while the Claude desktop app is the frontmost app and the Claude dictation is turned on, the system SHALL press Claude's dictation shortcut instead of recording; the hotkey press or release that ends the recording in the configured recording mode SHALL press the shortcut again, play the stop sound and hide the overlay, with nothing pasted; the user's Escape SHALL cancel it, which Claude hears itself, so the system SHALL press no second Escape. In any other frontmost app the hotkey SHALL record as before.

#### Scenario: Toggle mode in Claude
- **GIVEN** the recording mode is Toggle and the Claude desktop app is the frontmost app
- **WHEN** the user presses the hotkey, speaks, and presses it again
- **THEN** Claude's dictation shortcut is pressed at each hotkey press
- **AND** the start and stop sounds play and nothing is pasted by the app

#### Scenario: Hold mode in Claude
- **GIVEN** the recording mode is Hold and the Claude desktop app is the frontmost app
- **WHEN** the user holds the hotkey, speaks, and releases it
- **THEN** Claude's dictation shortcut is pressed at the press and again at the release

## MODIFIED Requirements

### Requirement: Holding the hotkey records and releasing it pastes
In Hold mode, pressing the hotkey SHALL start a recording, and releasing it SHALL end the recording, transcribe it and paste the text into the focused app; a press shorter than the shortest recording the app keeps SHALL record nothing and paste nothing. A Claude dictation SHALL follow its own requirements.

#### Scenario: A held hotkey records and a release pastes
- **GIVEN** the app is ready in Hold mode and a text field has keyboard focus
- **WHEN** the user holds the hotkey, speaks a phrase and releases the hotkey
- **THEN** a recording runs while the hotkey is held
- **AND** the phrase is transcribed and pasted into the text field after the release

#### Scenario: A short press records nothing
- **GIVEN** the app is ready in Hold mode
- **WHEN** the user releases the hotkey before the shortest recording the app keeps
- **THEN** nothing is transcribed or pasted
