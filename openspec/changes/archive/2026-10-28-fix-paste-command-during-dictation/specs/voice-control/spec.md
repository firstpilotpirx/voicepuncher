## ADDED Requirements

### Requirement: Each state accepts its own commands, paste in both
The system SHALL accept "start dictation" and "clear all text" only with no dictation running, "stop dictation", "cancel dictation" and "restart dictation" only during a dictation, and "press enter" and "paste text" in both. A command spoken in a state that does not accept it SHALL have no effect. "start" said during a dictation, however and wherever said, SHALL neither be taken for "restart" nor end the dictation.

#### Scenario: Paste during a dictation is carried out
- **GIVEN** a dictation runs
- **WHEN** the user says "paste text"
- **THEN** Cmd+V is pressed in the focused app
- **AND** the dictation keeps running

#### Scenario: Stop dictation without a dictation is ignored
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "stop dictation"
- **THEN** nothing is pasted and no sound plays

#### Scenario: Clear during a dictation is ignored
- **GIVEN** a dictation runs
- **WHEN** the user says "clear all text"
- **THEN** no key is pressed
- **AND** the dictation keeps running

#### Scenario: Start or let's start on its own during a dictation is ignored
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "start" or "let's start" on its own, at a normal level or in a quiet voice
- **THEN** no command takes effect
- **AND** the dictation keeps running

#### Scenario: Start ending a dictated sentence is ignored
- **GIVEN** a dictation runs
- **WHEN** the user says "we are all here, let's start" or "the meeting is about to start" without a pause, at a normal level or in a quiet voice, then pauses
- **THEN** no command takes effect
- **AND** the dictation keeps running

#### Scenario: Start inside a dictated sentence is ignored
- **GIVEN** a dictation runs
- **WHEN** the user says "let's start with the budget" or "we start at nine tomorrow" without pausing, at a normal level or in a quiet voice
- **THEN** no command takes effect
- **AND** the dictation keeps running

## MODIFIED Requirements

### Requirement: "paste text" pastes the clipboard
While listening, with or without a dictation running, the system SHALL press Cmd+V in the focused app when the user says "paste text", so the focused app pastes whatever the clipboard holds at its cursor: what the user copied, before the dictation or during it, never the dictated text. During a dictation the command SHALL NOT end, pause or restart the dictation: it keeps recording, and on its end pastes its own text, without the words of the command, where the cursor then is. The command SHALL leave the clipboard as it is. It SHALL then play the paste sound and show the paste sign on the overlay. With an empty clipboard the focused app pastes nothing, and the command still confirms itself.

#### Scenario: Saying paste text pastes the clipboard at the cursor
- **GIVEN** listening is on
- **AND** no dictation runs
- **AND** the clipboard holds a sentence
- **WHEN** the user says "paste text"
- **THEN** Cmd+V is pressed in the focused app
- **AND** the paste sound plays
- **AND** the overlay shows the paste sign for about a second, then hides

#### Scenario: Paste leaves the clipboard as it was
- **GIVEN** listening is on
- **AND** no dictation runs
- **AND** the clipboard holds a sentence
- **WHEN** the user says "paste"
- **THEN** the clipboard still holds the same sentence

#### Scenario: A paste phrase inside a sentence is ignored
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** someone says "I will paste the link later" without pausing
- **THEN** no key is pressed

#### Scenario: Saying paste during a dictation pastes the clipboard and the dictation goes on
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **AND** the clipboard holds another sentence the user copied
- **WHEN** the user says "paste text" with a pause before and after it
- **THEN** Cmd+V is pressed in the focused app
- **AND** the paste sound plays
- **AND** the dictation keeps running

#### Scenario: A dictation with a paste in it pastes its text without the command
- **GIVEN** a dictation runs
- **AND** the user has said a sentence, then "paste" on its own, then another sentence
- **WHEN** the user says "stop dictation"
- **THEN** both dictated sentences are pasted into the focused app
- **AND** the pasted text does not contain "paste"

## REMOVED Requirements

### Requirement: Each state accepts its own commands
**Reason**: Replaced by "Each state accepts its own commands, paste in both": "paste text" during a dictation now pastes the clipboard instead of being ignored, as the user asked.
**Migration**: None; the other commands keep their states.
