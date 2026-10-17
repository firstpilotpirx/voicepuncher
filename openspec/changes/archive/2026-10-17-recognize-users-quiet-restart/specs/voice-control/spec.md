## MODIFIED Requirements

### Requirement: The user's own way of saying the commands is recognized
The system SHALL recognize the commands as the user says them, including small slips of pronunciation or of the speech recognition service ("star dictation", "stuck dictation", "presenter", "and restart" for a quick "enter start" closing a chain). A command said on its own SHALL be recognized whatever the length of the pause before it, and in a quiet voice, as quiet as the user's quietest recorded commands, as well as at a normal level. Every command the user said in a recording kept for testing SHALL be recognized exactly once, apart from recordings marked unclear; repeats said without a pause of 0.3 s are one command. A short Russian phrase SHALL NOT be taken for a command; "стоп" is left out of this rule, since the speech recognition service may hear it as the short form "stop".

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

### Requirement: Each state accepts its own commands
The system SHALL accept "start dictation" and "clear all text" only while listening with no dictation running, "stop dictation", "cancel dictation" and "restart dictation" only during a dictation, and "press enter" in both. A command spoken in a state that does not accept it SHALL have no effect. "start" said during a dictation — on its own, ending a sentence ("let's start") or inside one, at a normal level or in a quiet voice — SHALL neither be taken for "restart" nor end the dictation.

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
