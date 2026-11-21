## ADDED Requirements

### Requirement: In Claude, voice commands drive Claude's own dictation
When a dictation starts by voice while the Claude desktop app is the frontmost app and the Claude dictation is turned on, the system SHALL press Claude's dictation shortcut instead of recording, and SHALL NOT record, transcribe, paste or archive that dictation; the start sound and the recording overlay SHALL be as for the app's own dictation. During such a dictation, "stop" SHALL press the shortcut again, play the stop sound and hide the overlay; "cancel" SHALL press Escape, play the cancel sound and hide the overlay; "restart" SHALL press Escape, then the shortcut, and play the restart sound; "press enter" SHALL press the shortcut, wait for Claude to insert its text, then press Return. In any other frontmost app a voice start SHALL start the app's own dictation as before. The choice SHALL be taken at each start and kept until that dictation ends.

#### Scenario: Saying start in Claude starts Claude's dictation
- **GIVEN** listening is on and the Claude dictation is turned on
- **AND** the Claude desktop app is the frontmost app
- **WHEN** the user says "start"
- **THEN** Claude's dictation shortcut is pressed once
- **AND** the start sound plays and the overlay shows the recording
- **AND** the app's own recorder does not start

#### Scenario: Saying stop in Claude stops Claude's dictation
- **GIVEN** a dictation started by voice in Claude runs
- **WHEN** the user says "stop"
- **THEN** Claude's dictation shortcut is pressed a second time
- **AND** the stop sound plays and the overlay hides
- **AND** nothing is pasted and no dictation is archived

#### Scenario: Saying cancel in Claude drops Claude's dictation
- **GIVEN** a dictation started by voice in Claude runs
- **WHEN** the user says "cancel"
- **THEN** Escape is pressed
- **AND** the cancel sound plays and the overlay hides

#### Scenario: Saying restart in Claude starts Claude's dictation over
- **GIVEN** a dictation started by voice in Claude runs
- **WHEN** the user says "restart"
- **THEN** Escape is pressed, then Claude's dictation shortcut
- **AND** the restart sound plays and the dictation goes on

#### Scenario: Saying enter in Claude stops and sends
- **GIVEN** a dictation started by voice in Claude runs
- **WHEN** the user says "enter"
- **THEN** Claude's dictation shortcut is pressed, then Return after a pause

#### Scenario: Another app keeps the app's own dictation
- **GIVEN** listening is on and the Claude dictation is turned on
- **AND** another app is the frontmost app
- **WHEN** the user says "start", a sentence, then "stop"
- **THEN** the app records and pastes the sentence as before
- **AND** Claude's dictation shortcut is never pressed

### Requirement: The Claude dictation falls back to the app's own dictation
The system SHALL start its own dictation, and log why, when the Claude dictation is turned off in the configuration, or when Claude's dictation shortcut cannot be pressed; a configured shortcut that is not Command with one letter or digit SHALL be refused when the configuration is loaded or saved.

#### Scenario: The Claude dictation turned off records as before
- **GIVEN** the Claude dictation is turned off
- **AND** the Claude desktop app is the frontmost app
- **WHEN** a dictation starts
- **THEN** the app's own recorder starts
- **AND** Claude's dictation shortcut is not pressed

#### Scenario: A shortcut that cannot be pressed falls back
- **GIVEN** the Claude desktop app is the frontmost app
- **AND** pressing Claude's dictation shortcut fails
- **WHEN** a dictation starts
- **THEN** the app's own recorder starts
- **AND** the log names why the Claude dictation was not used

#### Scenario: A shortcut without Command is refused
- **WHEN** the configuration sets the Claude dictation shortcut to "d"
- **THEN** loading the configuration fails, naming the shortcut
