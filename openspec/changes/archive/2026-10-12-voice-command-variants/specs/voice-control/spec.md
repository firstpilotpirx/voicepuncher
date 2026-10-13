# Spec Delta

## ADDED Requirements

### Requirement: A command may end with "please"
The system SHALL take each command phrase followed by "please" ("start dictation please", "press enter please") as that command, under the same rules as the phrase alone.

#### Scenario: Press enter please presses Return
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "press enter please"
- **THEN** the Return key is pressed in the focused app

#### Scenario: Stop dictation please pastes the dictated text
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop dictation please"
- **THEN** the sentence is pasted into the focused app

### Requirement: "clear text" erases the focused text too
The system SHALL take "clear text", with or without "please", as the "clear all text" command.

#### Scenario: Saying clear text selects all and deletes
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "clear text"
- **THEN** Cmd+A and then Delete are pressed in the focused app

### Requirement: A command said twice in one utterance counts once
The system SHALL carry out a command once when the user repeats it within one utterance, with a pause shorter than about half a second ("press enter, press enter"). A pause of half a second or more separates two commands.

#### Scenario: A repeated press enter presses Return once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "press enter, press enter" without pausing
- **THEN** the Return key is pressed once

### Requirement: The user's own way of saying the commands is recognized
The system SHALL recognize the commands as the user says them, including small slips of pronunciation ("star dictation", "presenter"). Every command the user said in a recording kept for testing SHALL be recognized exactly once, apart from recordings marked unclear; repeats said without a half-second pause are one command.

#### Scenario: Every recorded command of the user is recognized
- **GIVEN** the user's recordings of the commands, each labelled with the command said
- **WHEN** the recordings are heard one after another while listening, the app switching phrase sets as it does
- **THEN** each command said, repeats without a half-second pause counted as one, is recognized exactly once
- **AND** no other command takes effect

#### Scenario: A short Russian phrase is no command
- **GIVEN** voice control is listening
- **WHEN** someone says short Russian phrases ("да", "спасибо", "стоп"), each on its own with a pause around it
- **THEN** no command takes effect

### Requirement: A whole hands-free session runs as said
The system SHALL carry out a long hands-free session — dictations ended by "stop dictation" or "cancel dictation", Return presses and clears, in any order, with or without "please" — exactly as said: each stopped dictation pasted with its own words and without the command phrase, each cancelled one dropped, each key press done once, in order.

#### Scenario: A hands-free session in a synthesized voice
- **GIVEN** listening is on
- **WHEN** a synthesized voice dictates Russian and English sentences, sends one, cancels one, clears one and sends one again, each by voice command
- **THEN** each sent or cleared sentence is pasted with its words and no command phrase
- **AND** the Return presses, the clear and the cancelled dictation happen in the order said

#### Scenario: A hands-free session with the user's recorded commands
- **GIVEN** listening is on
- **AND** the user's own recordings of the commands exist on the machine
- **WHEN** the same session is spoken with the user's recorded commands
- **THEN** each sent or cleared sentence is pasted with its words and no command phrase
- **AND** the Return presses, the clear and the cancelled dictation happen in the order said

#### Scenario: A hands-free session with long pauses between its steps
- **GIVEN** listening is on
- **AND** the user's own recordings of the commands exist on the machine
- **WHEN** the session is spoken with the user's recorded commands and 15 seconds of room noise between its steps
- **THEN** each sent or cleared sentence is pasted with its words and no command phrase
- **AND** the Return presses, the clear and the cancelled dictation happen in the order said
