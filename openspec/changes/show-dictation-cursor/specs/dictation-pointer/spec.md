# Spec Delta

## Purpose

While a recording runs, a dictation sign next to the mouse pointer shows the user, where they already look, that the app is recording.

## ADDED Requirements

### Requirement: The dictation sign shows next to the pointer while a recording runs

From the moment a recording starts until it ends, the app SHALL show a microphone dictation sign just below and to the right of the mouse pointer, over any app, whether the recording was started by the hotkey or by voice. The dictation sign SHALL NOT show while processing, while waiting for focus, or while no dictation runs.

#### Scenario: A hotkey recording shows the dictation sign
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording with the hotkey
- **THEN** the dictation sign shows next to the pointer until the recording ends

#### Scenario: A recording started by voice shows the dictation sign
- **GIVEN** voice control is turned on and no dictation runs
- **WHEN** the user says "start dictation"
- **THEN** the dictation sign shows next to the pointer until the recording ends

#### Scenario: Processing shows no dictation sign
- **GIVEN** the user records a dictation with speech in it
- **WHEN** the user ends the recording
- **THEN** the dictation sign is gone while speech recognition, text refinement and the wait for focus run

### Requirement: The dictation sign goes away as soon as the recording ends

The dictation sign SHALL go away within a moment of the recording's end, however it ends: stopped by the hotkey or by voice, cancelled, dropped as too short, or the app quits during it. A dictation that restarts SHALL show the sign again for the new recording.

#### Scenario: Stopping a recording removes the dictation sign
- **GIVEN** a recording runs and the dictation sign shows
- **WHEN** the user ends the recording with the hotkey or by saying "stop dictation"
- **THEN** the dictation sign is gone

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
