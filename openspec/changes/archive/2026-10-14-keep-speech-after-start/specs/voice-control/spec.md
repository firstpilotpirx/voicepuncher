## ADDED Requirements

### Requirement: A dictation started by voice holds the speech right after the command
When a voice command starts a dictation ("start dictation" or "start" said on its own, opening an utterance, doubled, or last in a chain, or "restart"), the system SHALL begin the dictation where the command's words end, not when the command was recognized. So all the speech said after the command, its first words included, SHALL be part of the dictation. The command words SHALL NOT be pasted.

#### Scenario: Start start followed by the dictation at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and a sentence without a pause, and later ends the dictation
- **THEN** the whole sentence, its first words included, is pasted into the focused app without "start"

#### Scenario: Start, a short pause, then the dictation
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start", pauses for about 0.5 s and says a sentence, and later ends the dictation
- **THEN** the whole sentence, its first words included, is pasted into the focused app without "start"

#### Scenario: Restart, a short pause, then the new dictation
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "restart", pauses for about 0.5 s and says another sentence, and later ends the dictation
- **THEN** only the other sentence is pasted, its first words included

#### Scenario: A chain ending in start followed by the next dictation at once
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop enter start" and another sentence without a pause, and later ends the dictation
- **THEN** the other sentence, its first words included, is pasted after the Return key is pressed

#### Scenario: The user's recorded restart restart followed by the new dictation at once
- **GIVEN** the user's own recordings of the commands exist on the machine
- **AND** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user's recorded "restart restart" is followed at once by another sentence, and the dictation later ends
- **THEN** only the other sentence is pasted, its first words included

#### Scenario: The user's recorded start start followed by the dictation at once
- **GIVEN** the user's own recordings of the commands exist on the machine
- **AND** listening is on
- **WHEN** the user's recorded "start start" is followed at once by a sentence, and the dictation later ends
- **THEN** the whole sentence, its first words included, is pasted into the focused app without "start"
