# Spec Delta

## ADDED Requirements

### Requirement: "paste text" pastes the clipboard
While listening and no dictation runs, the system SHALL press Cmd+V in the focused app when the user says "paste text", so the focused app pastes whatever the clipboard holds at its cursor. The command SHALL leave the clipboard as it is. It SHALL then play the paste sound and show the paste sign on the overlay. With an empty clipboard the focused app pastes nothing, and the command still confirms itself.

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

## MODIFIED Requirements

### Requirement: Each state accepts its own commands
The system SHALL accept "start dictation", "clear all text" and "paste text" only while listening with no dictation running, "stop dictation", "cancel dictation" and "restart dictation" only during a dictation, and "press enter" in both. A command spoken in a state that does not accept it SHALL have no effect. "start" said during a dictation — on its own, ending a sentence ("let's start") or inside one, at a normal level or in a quiet voice — SHALL neither be taken for "restart" nor end the dictation.

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

#### Scenario: Paste during a dictation is ignored
- **GIVEN** a dictation runs
- **WHEN** the user says "paste text"
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

### Requirement: Every command has a one-word short form
The system SHALL take "start" as "start dictation", "stop" as "stop dictation", "cancel" as "cancel dictation", "restart" as "restart dictation", "enter" as "press enter", "clear" as "clear all text" and "paste" as "paste text", with or without a trailing "please", under the same rules as the full phrase, in the state that accepts the command.

#### Scenario: A short form is recognized as its command
- **GIVEN** listening is on
- **WHEN** the user says "start", "stop", "cancel", "restart", "enter", "clear" or "paste" on its own, in the state that accepts its command
- **THEN** "start dictation", "stop dictation", "cancel dictation", "restart dictation", "press enter", "clear all text" or "paste text" takes effect

#### Scenario: Cancel alone discards a dictation
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "cancel"
- **THEN** no text is pasted

### Requirement: A doubled short form counts anywhere
The system SHALL carry out a command at once when its short form is said twice in a row — "start start", "stop stop", "cancel cancel", "enter enter", "clear clear", "paste paste" — anywhere in an utterance, with no pause around it, in a state that accepts the command. When "stop stop" or "cancel cancel" ends a dictation, the words after it in the same utterance SHALL NOT be pasted; with "stop stop" the words before it SHALL be pasted.

#### Scenario: Stop stop in the middle of speech
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence, "stop stop" and more words, without any pause
- **THEN** the sentence is pasted into the focused app
- **AND** neither "stop" nor the words after it are pasted

#### Scenario: Enter enter in the middle of speech
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** someone says "fine enter enter thanks" without a pause
- **THEN** the Return key is pressed once

#### Scenario: Cancel cancel in the middle of speech
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence and "cancel cancel" without a pause
- **THEN** no text is pasted

### Requirement: Which chains exist
Since "start dictation" leads to dictating and "stop dictation" or "cancel dictation" lead back to standby, the chains SHALL be exactly those where "start" comes last: during a dictation, "stop" or "cancel" followed by one or two of "enter", "clear", "paste", "start" (each once, "start" last), or "enter" followed by one or two of "clear", "paste", "start" ("enter" ends the dictation and pastes first); with no dictation, two or three of "enter", "clear", "paste", "start" (each once, "start" last).

#### Scenario: Stop clear enter
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop clear enter"
- **THEN** the sentence is pasted, then select all and delete are pressed, then the Return key is pressed

#### Scenario: Paste enter pastes the clipboard and sends
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "paste enter"
- **THEN** Cmd+V is pressed, then the Return key is pressed

### Requirement: Every confirmed command sounds different
The enter, clear, paste and restart sounds SHALL differ from each other and from the start, stop, cancel, listening-on and listening-off sounds, unless the user picks the same sound for two of them. Each of the four SHALL be the user's to change, like the other sounds, with a distinct sound by default.

#### Scenario: Enter and clear sound different by default
- **GIVEN** the user has not changed any sound
- **WHEN** the user says "enter", and later "clear"
- **THEN** the two commands play different sounds

#### Scenario: Paste sounds different from enter and clear by default
- **GIVEN** the user has not changed any sound
- **WHEN** the user says "paste", and later "enter" and "clear"
- **THEN** the paste sound differs from the enter and clear sounds

#### Scenario: A sound the user picked for a command plays
- **GIVEN** the user picked another sound for "clear all text"
- **WHEN** the user says "clear all text"
- **THEN** the picked sound plays

### Requirement: A confirmation sign gives way to what comes next
The enter, clear, paste and restart signs SHALL each name the command that ran. A sign SHALL give way at once to what the overlay shows next — the recording of a dictation a chain starts, or the sign of the next command in the chain — and after about a second the overlay SHALL return to what it showed before the command: hidden in standby, the recording during a dictation.

#### Scenario: Enter start shows the recording right after the enter sign
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter start"
- **THEN** the enter sound plays and the overlay shows the enter sign
- **AND** the overlay shows the recording as soon as the new dictation starts

#### Scenario: Stop clear enter ends on the enter sign
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop clear enter"
- **THEN** the stop sound, the clear sound and the enter sound play in this order
- **AND** the overlay shows the clear sign, then the enter sign, then hides
