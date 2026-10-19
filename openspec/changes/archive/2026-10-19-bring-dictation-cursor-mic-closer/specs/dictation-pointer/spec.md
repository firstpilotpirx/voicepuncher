## MODIFIED Requirements

### Requirement: The dictation sign shows next to the pointer while a recording runs

From the moment a recording starts until it ends, the app SHALL show a microphone dictation sign just below and to the right of the mouse pointer, over any app, whether the recording was started by the hotkey or by voice. The sign SHALL sit close to the pointer's tip, so it reads as attached to the pointer: its top-left corner 8 points to the right of the tip and 6 points below it. The dictation sign SHALL NOT show while processing, while waiting for focus, or while no dictation runs.

#### Scenario: A hotkey recording shows the dictation sign
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording with the hotkey
- **THEN** the dictation sign shows next to the pointer until the recording ends

#### Scenario: A recording started by voice shows the dictation sign
- **GIVEN** voice control is turned on and no dictation runs
- **WHEN** the user says "start dictation"
- **THEN** the dictation sign shows next to the pointer until the recording ends

#### Scenario: The dictation sign sits close to the pointer's tip
- **GIVEN** a recording runs
- **WHEN** the dictation sign shows next to the pointer
- **THEN** its top-left corner sits 8 points right of and 6 points below the pointer's tip, not the 16 points right and 16 points below it sat before

#### Scenario: Processing shows no dictation sign
- **GIVEN** the user records a dictation with speech in it
- **WHEN** the user ends the recording
- **THEN** the dictation sign is gone while speech recognition, text refinement and the wait for focus run
