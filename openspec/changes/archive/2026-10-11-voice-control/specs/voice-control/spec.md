# Spec Delta

## Purpose

Lets the user dictate hands-free: with voice control on, the hotkey turns listening on and off, and five spoken English commands start, stop and cancel a dictation, press Enter and clear the text in the focused app.

## ADDED Requirements

### Requirement: Voice control is opt-in
The system SHALL keep voice control off unless the config file or a command-line flag turns it on. With voice control off, the hotkey SHALL record as push-to-talk does today. With voice control on, a hotkey press SHALL NOT start a recording; it SHALL control listening.

#### Scenario: The hotkey records when voice control is off
- **GIVEN** voice control is not turned on
- **WHEN** the user presses the hotkey
- **THEN** a recording starts

#### Scenario: The hotkey does not record when voice control is on
- **GIVEN** voice control is turned on
- **AND** listening is off
- **WHEN** the user presses the hotkey
- **THEN** no recording starts
- **AND** listening is on

### Requirement: The hotkey turns listening on and off
With voice control on, the system SHALL start listening as soon as the app starts, without a hotkey press. A hotkey press SHALL turn listening off when it is on and no dictation runs, and on again when it is off. While listening, the microphone SHALL stay open and the system SHALL react to spoken commands. Turning listening on and off SHALL each play its own sound.

#### Scenario: Listening is on at start
- **GIVEN** voice control is turned on
- **WHEN** the app starts
- **THEN** listening is on
- **AND** the listening-on sound plays

#### Scenario: A hotkey press turns listening on
- **GIVEN** voice control is turned on
- **AND** listening is off
- **WHEN** the user presses the hotkey
- **THEN** listening is on
- **AND** the listening-on sound plays

#### Scenario: A hotkey press turns listening off
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user presses the hotkey
- **THEN** listening is off
- **AND** the listening-off sound plays
- **AND** spoken commands have no effect

### Requirement: "start dictation" starts a dictation
While listening and no dictation runs, the system SHALL start a dictation when the user says "start dictation", exactly as a hotkey press starts a recording in push-to-talk, with the same start sound and overlay.

#### Scenario: Saying start dictation starts a dictation
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start dictation"
- **THEN** a dictation starts
- **AND** the start sound plays

### Requirement: "stop dictation" ends a dictation and pastes its text
During a dictation, the system SHALL end the dictation when the user says "stop dictation", then transcribe, refine and paste the text as a push-to-talk recording does. The spoken words "stop dictation" SHALL NOT appear in the pasted text. Listening SHALL stay on afterwards.

#### Scenario: Saying stop dictation pastes the dictated text
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop dictation"
- **THEN** the sentence is pasted into the focused app
- **AND** the pasted text does not contain the stop phrase
- **AND** listening stays on

### Requirement: "cancel dictation" discards a dictation
During a dictation, the system SHALL discard the dictation when the user says "cancel dictation": nothing is pasted and the cancel sound plays, as when a push-to-talk recording is cancelled. Listening SHALL stay on afterwards.

#### Scenario: Saying cancel dictation pastes nothing
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "cancel dictation"
- **THEN** nothing is pasted
- **AND** the cancel sound plays
- **AND** listening stays on

### Requirement: "press enter" presses Return
While listening and no dictation runs, the system SHALL press the Return key in the focused app when the user says "press enter".

#### Scenario: Saying press enter presses Return
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "press enter"
- **THEN** the Return key is pressed in the focused app
- **AND** the command sound plays

### Requirement: "clear all text" erases the focused text
While listening and no dictation runs, the system SHALL select all and delete in the focused app when the user says "clear all text": it presses Cmd+A, then Delete.

#### Scenario: Saying clear all text selects all and deletes
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "clear all text"
- **THEN** Cmd+A and then Delete are pressed in the focused app
- **AND** the command sound plays

### Requirement: A command counts only when spoken on its own
The system SHALL carry out a command only when its phrase is the whole utterance: a pause before it and a pause after it. The same words SHALL NOT be taken as a command when more than about 0.3 s of speech comes before them or more than about 0.5 s follows them in the utterance; a single short word right before or after a command ("please press enter", "press enter now") is within the spotter's timing error and may still count.

#### Scenario: A command phrase spoken on its own is recognized
- **GIVEN** listening is on
- **WHEN** the user says a command phrase with a pause before and after it
- **THEN** that command takes effect

#### Scenario: A command said after a long time of listening is recognized
- **GIVEN** listening has been on for a long time with nothing said
- **WHEN** the user says a command phrase with a pause before and after it
- **THEN** that command takes effect

#### Scenario: A command phrase inside a sentence is ignored while listening
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** someone says "I will press enter when I am done" without pausing
- **THEN** no key is pressed
- **AND** no dictation starts

#### Scenario: Ordinary speech triggers no command
- **GIVEN** listening is on
- **WHEN** someone speaks ordinary Russian and English sentences that hold no command phrase
- **THEN** no command takes effect

#### Scenario: A stop phrase inside a dictated sentence does not end the dictation
- **GIVEN** a dictation runs
- **WHEN** the user says "please stop dictation software from crashing" without pausing
- **THEN** the dictation keeps running

### Requirement: Each state accepts only its own commands
The system SHALL accept "start dictation", "press enter" and "clear all text" only while listening with no dictation running, and "stop dictation" and "cancel dictation" only during a dictation. A command spoken in another state SHALL have no effect.

#### Scenario: Press enter during a dictation is ignored
- **GIVEN** a dictation runs
- **WHEN** the user says "press enter"
- **THEN** no key is pressed
- **AND** the dictation keeps running

#### Scenario: Stop dictation without a dictation is ignored
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "stop dictation"
- **THEN** nothing is pasted and no sound plays

### Requirement: Commands are carried out in the order spoken
The system SHALL carry out commands in the order they were spoken. A command spoken while the previous dictation is still being transcribed, refined or pasted SHALL wait until that text is pasted.

#### Scenario: Press enter right after stop dictation follows the paste
- **GIVEN** a dictation runs
- **AND** the user has said "stop dictation"
- **WHEN** the user says "press enter" before the text is pasted
- **THEN** the text is pasted first
- **AND** the Return key is pressed after it

#### Scenario: A message is dictated and sent by voice alone
- **GIVEN** listening is on
- **WHEN** the user says "start dictation", a sentence, "stop dictation" and "press enter", each command on its own
- **THEN** the sentence is pasted into the focused app
- **AND** the Return key is pressed after it

### Requirement: The hotkey during a dictation ends it and turns listening off
With voice control on, a hotkey press during a dictation SHALL end the dictation as "stop dictation" does, paste its text, and then turn listening off.

#### Scenario: A hotkey press during a dictation pastes and stops listening
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user presses the hotkey
- **THEN** the sentence is pasted into the focused app
- **AND** listening is off

### Requirement: Voice control needs the command spotting service
With voice control on, the system SHALL refuse to start, with a message naming the command spotting service and how to start it, when that service does not answer. When the service stops answering while listening, the system SHALL log an error and turn listening off.

#### Scenario: The app refuses to start without the command spotting service
- **GIVEN** voice control is turned on
- **AND** the command spotting service is not running
- **WHEN** the app starts
- **THEN** the app exits with an error that names the command spotting service and how to start it

#### Scenario: Listening turns off when the command spotting service goes away
- **GIVEN** listening is on
- **WHEN** the command spotting service stops answering
- **THEN** an error is logged
- **AND** listening is off
