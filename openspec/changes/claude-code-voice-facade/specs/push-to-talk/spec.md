## ADDED Requirements

### Requirement: In Claude, the hotkey drives Claude's own dictation
When the hotkey starts a recording while the Claude desktop app is the frontmost app and the Claude dictation is turned on, the system SHALL press Claude's dictation shortcut instead of recording; the hotkey press or release that ends the recording in the configured recording mode SHALL press the shortcut again, play the stop sound and hide the overlay, with nothing pasted; cancelling SHALL press Escape. In any other frontmost app the hotkey SHALL record as before.

#### Scenario: Toggle mode in Claude
- **GIVEN** the recording mode is Toggle and the Claude desktop app is the frontmost app
- **WHEN** the user presses the hotkey, speaks, and presses it again
- **THEN** Claude's dictation shortcut is pressed at each hotkey press
- **AND** the start and stop sounds play and nothing is pasted by the app

#### Scenario: Hold mode in Claude
- **GIVEN** the recording mode is Hold and the Claude desktop app is the frontmost app
- **WHEN** the user holds the hotkey, speaks, and releases it
- **THEN** Claude's dictation shortcut is pressed at the press and again at the release
