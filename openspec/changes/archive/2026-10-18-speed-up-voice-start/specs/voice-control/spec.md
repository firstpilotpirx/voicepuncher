# Spec Delta

## ADDED Requirements

### Requirement: A voice start gives its start cue within 0.7 s
When a voice command starts a dictation from standby, or "restart" starts one over, the system SHALL play the sound that start plays (the restart sound for "restart") as soon as the command is confirmed, within 0.7 s of the end of the command's words in at least 9 of 10 starts, with the services otherwise idle; the overlay's recording sign and the recording follow once the microphone opens. In the scenarios below "the start cue" means that sound. This holds for "start" alone, opening or ending an utterance, or doubled, and every form of "restart".

#### Scenario: Start start followed by the dictation at once gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and a sentence without a pause
- **THEN** the start cue comes within 0.7 s of the end of "start start", while the sentence is still being said

#### Scenario: Start followed by the dictation at once gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start" and a sentence without a pause
- **THEN** the start cue comes within 0.7 s of the end of "start", while the sentence is still being said

#### Scenario: Start said on its own gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start" and stays silent
- **THEN** the start cue comes within 0.7 s of the end of "start"

#### Scenario: Restart restart followed by the new dictation at once gives its cue at once
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "restart restart" and another sentence without a pause
- **THEN** the start cue of the new dictation comes within 0.7 s of the end of "restart restart"

#### Scenario: Restart said on its own gives its cue at once
- **GIVEN** a dictation runs
- **AND** the user has dictated for about 20 s
- **WHEN** the user says "restart" and stays silent
- **THEN** the start cue of the new dictation comes within 0.7 s of the end of "restart"

#### Scenario: The user's recorded starts give their cue at once
- **GIVEN** the user's own recordings of the commands exist on the machine
- **AND** listening is on
- **WHEN** each of the user's recorded starts is played, on its own and followed at once by a sentence
- **THEN** at least 9 of 10 of them give the start cue within 0.7 s of the end of the command's words

#### Scenario: A faster start keeps the dictation where the command's words end
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and a sentence without a pause, and later ends the dictation
- **THEN** the whole sentence, its first words included, is pasted into the focused app without "start"

#### Scenario: A faster start adds no false start
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says a sentence with "start" inside it, ordinary speech, a short Russian phrase or "start stop"
- **THEN** no dictation starts

### Requirement: Every voice start logs how long its start cue took
For every voice start in scope of the start cue bound, the system SHALL log one line with the time from the end of the command's words to the start cue and the time of each stage on the way: hearing the command, confirming it, delivering it to the app, and giving the cue. The time SHALL stay right after part of the microphone audio never reached the command spotting service.

#### Scenario: A voice start logs its timing
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and a sentence without a pause
- **THEN** the log shows one timing line for that start, with its total and the time of each stage

#### Scenario: The timing stays right after microphone audio was lost
- **GIVEN** listening is on
- **AND** some seconds of microphone audio never reached the command spotting service earlier
- **WHEN** the user says "start" and stays silent
- **THEN** the logged total matches the time from the end of "start" to the start cue within 0.1 s

#### Scenario: A hotkey start logs no voice start timing
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user starts a dictation with the hotkey
- **THEN** the log shows no voice start timing line
