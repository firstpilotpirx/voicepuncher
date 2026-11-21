## ADDED Requirements

### Requirement: In Claude, the hotkey drives Claude's own dictation
When the hotkey starts a recording while the Claude desktop app is frontmost and the Claude dictation is on, the system SHALL press Claude's dictation shortcut instead of recording; the press or release that ends it in the recording mode SHALL press the shortcut again, play the stop sound and hide the overlay, pasting nothing; the user's Escape, which Claude hears itself, SHALL cancel it with no second Escape. In any other app the hotkey SHALL record as before.

#### Scenario: Toggle mode in Claude
- **GIVEN** the recording mode is Toggle and the Claude desktop app is the frontmost app
- **WHEN** the user presses the hotkey, speaks, and presses it again
- **THEN** Claude's dictation shortcut is pressed at each hotkey press
- **AND** the start and stop sounds play and nothing is pasted by the app

#### Scenario: Hold mode in Claude
- **GIVEN** the recording mode is Hold and the Claude desktop app is the frontmost app
- **WHEN** the user holds the hotkey, speaks, and releases it
- **THEN** Claude's dictation shortcut is pressed at the press and again at the release

#### Scenario: Escape in Claude leaves the drop to Claude
- **GIVEN** the Claude dictation is turned on and the Claude desktop app is the frontmost app
- **WHEN** the user presses the hotkey once, then Escape
- **THEN** the system presses no Escape of its own
- **AND** the cancel sound plays

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
