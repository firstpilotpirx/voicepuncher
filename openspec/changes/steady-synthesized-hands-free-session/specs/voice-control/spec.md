## MODIFIED Requirements

### Requirement: A command may end with "please"
The system SHALL take each command phrase followed by "please" ("start dictation please", "press enter please") as that command, under the same rules as the phrase alone. A "please" that opens a dictation started by "start dictation" or "restart dictation" belongs to the command and SHALL NOT be pasted, even when the command was recognized before the "please" was heard.

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


#### Scenario: Please after start dictation is not pasted
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start dictation please" and a sentence, the start being recognized before the "please" was heard, and later ends the dictation
- **THEN** the sentence is pasted into the focused app
- **AND** the pasted text does not contain "please"

### Requirement: The user's own way of saying the commands is recognized
The system SHALL recognize the commands as the user says them, including small slips of pronunciation or of the speech recognition service ("star dictation", "stuck dictation", "presenter", "capsule dictation" for "cancel dictation", "and restart" for a quick "enter start" closing a chain). A command said on its own SHALL be recognized whatever the length of the pause before it, and in a quiet voice, as quiet as the user's quietest recorded commands, as well as at a normal level. Every command the user said in a recording kept for testing SHALL be recognized exactly once, apart from recordings marked unclear; repeats said without a pause of 0.3 s are one command. A short Russian phrase SHALL NOT be taken for a command; "стоп" is left out of this rule, since the speech recognition service may hear it as the short form "stop".

#### Scenario: Every recorded command of the user is recognized
- **GIVEN** the user's recordings of the commands, each labelled with the command said
- **WHEN** the recordings are heard one after another while listening, the app switching phrase sets as it does
- **THEN** each command said, repeats without a 0.3 s pause counted as one, is recognized exactly once
- **AND** no other command takes effect

#### Scenario: A short Russian phrase is no command
- **GIVEN** voice control is listening
- **WHEN** someone says short Russian phrases ("да", "спасибо", "привет"), each on its own with a pause around it
- **THEN** no command takes effect

#### Scenario: The user's quiet restart after any pause before it
- **GIVEN** the user's own recordings of the commands exist on the machine, among them a "restart" said in a quiet voice
- **AND** a dictation runs
- **WHEN** the user's quiet recorded "restart" is heard on its own after a silence of any length from 0.3 to 0.8 s
- **THEN** the dictation starts over

#### Scenario: The user's quiet restart, a short pause, then speech
- **GIVEN** the user's own recordings of the commands exist on the machine, among them a "restart" said in a quiet voice
- **AND** a dictation runs
- **WHEN** the user's quiet recorded "restart", said on its own, is followed by 0.3–0.6 s of silence and then speech
- **THEN** the dictation starts over once
- **AND** no other command takes effect

#### Scenario: A command said in a quiet voice is recognized
- **GIVEN** listening is on
- **WHEN** the user says a command phrase or its short form on its own in a quiet voice, in the state that accepts the command
- **THEN** that command takes effect

#### Scenario: A quiet clear all text clears the text
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "clear all text" on its own, as quiet as the user's quietest recorded commands, with a pause before and after it
- **THEN** the focused text is selected and deleted once


#### Scenario: Cancel dictation heard as capsule dictation discards the dictation
- **GIVEN** a dictation runs
- **WHEN** the user says "cancel dictation" on its own and the speech recognition service hears it as "Capsule dictation"
- **THEN** the dictation is discarded and nothing is pasted
