# dictation-pointer Specification

## Purpose
While a dictation runs, a dictation sign next to the mouse pointer shows the user, where they already look, what the app is doing: recording, processing, waiting for focus, pasted or failed.

## Requirements

### Requirement: The dictation sign follows the pointer without getting in the way

While it shows, the dictation sign SHALL move with the pointer on every screen, keeping its place next to it. It SHALL NOT take clicks, scrolling, keyboard focus or pointer movement from any app: everything under it works as if it were not there.

#### Scenario: The dictation sign follows the pointer to another screen
- **GIVEN** two screens are connected and a recording runs
- **WHEN** the user moves the pointer to the other screen
- **THEN** the dictation sign shows next to the pointer on that screen

#### Scenario: Clicks pass through the dictation sign
- **GIVEN** a recording runs and the dictation sign covers part of a button
- **WHEN** the user clicks the button through the dictation sign
- **THEN** the button is pressed and the focused app keeps its focus

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

### Requirement: The dictation sign goes away once the dictation is over

The dictation sign SHALL go away within a moment of the overlay hiding after a dictation, however it ends: pasted, held back and left in the clipboard, cancelled, dropped as too short, failed, or the app quits during it. It SHALL NOT show while no dictation runs and no failure notice shows, nor for the commands-unavailable sign alone. A dictation that restarts SHALL show the sign again for the new recording.

#### Scenario: A pasted dictation removes the dictation sign with the overlay
- **GIVEN** a recording runs and the dictation sign shows
- **WHEN** the user ends the recording and the text is pasted
- **THEN** the dictation sign is gone when the overlay hides after the Pasted sign

#### Scenario: Cancelling a recording removes the dictation sign
- **GIVEN** a recording runs and the dictation sign shows
- **WHEN** the user cancels the dictation
- **THEN** the dictation sign is gone

#### Scenario: A recording too short to keep removes the dictation sign
- **GIVEN** the user presses the hotkey
- **WHEN** the user releases it before the shortest recording the app keeps
- **THEN** the dictation sign is gone

#### Scenario: Restarting a dictation shows the sign for the new recording
- **GIVEN** voice control is turned on and a recording runs
- **WHEN** the user says "restart dictation"
- **THEN** the dictation sign shows next to the pointer for the new recording

#### Scenario: Quitting the app during a recording leaves the normal pointer
- **GIVEN** a recording runs and the dictation sign shows
- **WHEN** the app quits
- **THEN** the dictation sign is gone and the pointer looks as usual

#### Scenario: Commands unavailable in standby show no dictation sign
- **GIVEN** listening is on and no dictation runs
- **WHEN** the command spotting service stays away for longer than 5 seconds
- **THEN** the overlay shows the commands-unavailable sign and no dictation sign shows
