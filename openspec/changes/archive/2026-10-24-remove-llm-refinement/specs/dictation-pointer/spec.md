## MODIFIED Requirements

### Requirement: The dictation sign shows next to the pointer while a dictation runs

From the moment a recording starts until the overlay hides, the app SHALL show the dictation sign just below and to the right of the mouse pointer, over any app, whether the recording was started by the hotkey or by voice: a small round dark-glass badge that mirrors the overlay's state. The sign SHALL sit close to the pointer's tip, its top-left corner 8 points right of the tip and 6 points below it. The system pointer SHALL stay as it is.

#### Scenario: A hotkey recording shows the dictation sign
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording with the hotkey
- **THEN** the dictation sign shows next to the pointer with a red microphone

#### Scenario: A recording started by voice shows the dictation sign
- **GIVEN** voice control is turned on and no dictation runs
- **WHEN** the user says "start dictation"
- **THEN** the dictation sign shows next to the pointer with a red microphone

#### Scenario: The dictation sign sits close to the pointer's tip
- **GIVEN** a recording runs
- **WHEN** the dictation sign shows next to the pointer
- **THEN** its top-left corner sits 8 points right of and 6 points below the pointer's tip

#### Scenario: The dictation sign mirrors every stage of a dictation
- **GIVEN** a button has keyboard focus
- **WHEN** the user records a dictation with speech in it, ends it and then clicks into a text field
- **THEN** the dictation sign shows a red microphone, then an ear with a turning ring, then an orange keyboard, then a green check
- **AND** the system pointer looks as usual throughout

#### Scenario: A failure shows a warning triangle in the dictation sign
- **GIVEN** the speech recognition service is not running
- **WHEN** the user stops a dictation
- **THEN** the dictation sign shows a yellow warning triangle until the failure notice hides

#### Scenario: A command sign leaves the dictation sign as it was
- **GIVEN** voice control is turned on and a dictation runs
- **WHEN** the user says "clear text"
- **THEN** the dictation sign keeps showing the red microphone
