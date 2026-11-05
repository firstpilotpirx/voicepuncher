# Spec Delta

## MODIFIED Requirements

### Requirement: A chain of commands is carried out in order
The system SHALL carry out a chain of two or three different commands said one right after the other — full phrases or short forms — in the order said, when each command is accepted in the state the one before it leads to and "start" comes last. A chain SHALL count anywhere in an utterance, with no pause around it. A command of the chain heard only after the commands before it were carried out SHALL still be carried out, once, after them, whichever recognizer hears it: the chain's tail is never lost.

#### Scenario: Stop enter pastes and sends
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop enter"
- **THEN** the sentence is pasted into the focused app
- **AND** the Return key is pressed after it

#### Scenario: Stop enter start sends and starts the next dictation
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop enter start"
- **THEN** the sentence is pasted into the focused app
- **AND** the Return key is pressed after it
- **AND** a new dictation starts

#### Scenario: Cancel clear discards and clears
- **GIVEN** a dictation runs
- **WHEN** the user says "cancel clear"
- **THEN** no text is pasted
- **AND** select all and delete are pressed in the focused app

#### Scenario: A chain the states do not allow is no command
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter stop" or "start stop"
- **THEN** no command takes effect

#### Scenario: The end of a chain heard only when the utterance ends still counts
- **GIVEN** a dictation runs
- **AND** while the user speaks, the system has heard only the first two commands of "never mind cancel enter start" and carried them out
- **WHEN** the utterance ends and the system hears all of it
- **THEN** "start dictation" is carried out after them
- **AND** "cancel dictation" and "press enter" are not carried out again

#### Scenario: The last command of a chain decoded after the first ones were carried out still counts
- **GIVEN** a dictation runs
- **AND** the user says "that is all stop enter start" in one breath
- **AND** a recognizer hears "stop enter" and the system carries them out before any recognizer has heard "start"
- **WHEN** a recognizer hears the "start" said right after them, while the user still speaks or at the close
- **THEN** "start dictation" is carried out once, after them
- **AND** "stop dictation" and "press enter" are not carried out again

#### Scenario: The user's recorded chains keep their last command
- **GIVEN** a dictation runs
- **WHEN** each of the user's recorded chains whose last command counts only after the one before it ("start" or "clear" while dictating) is played
- **THEN** every command of the chain is carried out, in the order said, each once
