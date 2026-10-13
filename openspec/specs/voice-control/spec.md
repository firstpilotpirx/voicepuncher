# voice-control Specification

## Purpose
Lets the user dictate hands-free alongside push-to-talk: with voice control on, listening stays on while the app runs, the hotkey still records as push-to-talk, and five spoken English commands (with one-word short forms) start, stop and cancel a dictation, press Enter and clear the text in the focused app.

## Requirements

### Requirement: "start dictation" starts a dictation
While listening and no dictation runs, the system SHALL start a dictation when the user says "start dictation", exactly as a hotkey press starts a recording in push-to-talk, with the same start sound and overlay. Ending it by voice or by hotkey SHALL play the stop sound and show the processing overlay, and cancelling it SHALL play the cancel sound and hide the overlay, as in push-to-talk.

#### Scenario: Saying start dictation starts a dictation
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start dictation"
- **THEN** a dictation starts
- **AND** the start sound plays
- **AND** the overlay shows the recording

#### Scenario: Saying stop dictation plays the stop sound and shows processing
- **GIVEN** the user started a dictation by voice and said a sentence
- **WHEN** the user says "stop dictation"
- **THEN** the stop sound plays
- **AND** the overlay shows processing until the sentence is pasted, then hides

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
The system SHALL recognize the commands as the user says them, including small slips of pronunciation ("star dictation", "presenter"). Every command the user said in a recording kept for testing SHALL be recognized exactly once, apart from recordings marked unclear; repeats said without a half-second pause are one command. A short Russian phrase SHALL NOT be taken for a command; "стоп" is left out of this rule, since the speech recognition service may hear it as the short form "stop".

#### Scenario: Every recorded command of the user is recognized
- **GIVEN** the user's recordings of the commands, each labelled with the command said
- **WHEN** the recordings are heard one after another while listening, the app switching phrase sets as it does
- **THEN** each command said, repeats without a half-second pause counted as one, is recognized exactly once
- **AND** no other command takes effect

#### Scenario: A short Russian phrase is no command
- **GIVEN** voice control is listening
- **WHEN** someone says short Russian phrases ("да", "спасибо", "привет"), each on its own with a pause around it
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

### Requirement: Push-to-talk works with voice control on or off
The system SHALL keep voice control off unless the config file or a command-line flag turns it on. Whether voice control is on or off, the hotkey SHALL record as push-to-talk does: a press starts a dictation, a press in toggle mode or the release in hold mode ends it and pastes its text, and Escape cancels it.

#### Scenario: The hotkey records when voice control is off
- **GIVEN** voice control is not turned on
- **WHEN** the user presses the hotkey
- **THEN** a recording starts

#### Scenario: The hotkey records when voice control is on
- **GIVEN** voice control is turned on
- **AND** no dictation runs
- **WHEN** the user presses the hotkey
- **THEN** a dictation starts
- **AND** the start sound plays
- **AND** the overlay shows the recording

#### Scenario: The hotkey ends its dictation when voice control is on
- **GIVEN** voice control is turned on
- **AND** the user started a dictation with the hotkey and said a sentence
- **WHEN** the user presses the hotkey again
- **THEN** the sentence is pasted into the focused app
- **AND** listening stays on

#### Scenario: Escape cancels a dictation when voice control is on
- **GIVEN** voice control is turned on
- **AND** a dictation runs
- **WHEN** the user presses Escape
- **THEN** nothing is pasted
- **AND** the cancel sound plays
- **AND** the overlay hides
- **AND** listening stays on

### Requirement: The hotkey ends a dictation started by voice
With voice control on, a hotkey press in toggle mode, or its release in hold mode, during a dictation SHALL end the dictation as "stop dictation" does and paste its text, with the stop sound, whether the hotkey or "start dictation" started it. Listening SHALL stay on.

#### Scenario: A hotkey press ends a voice dictation and keeps listening
- **GIVEN** the user started a dictation by saying "start dictation"
- **AND** the user has said a sentence in it
- **WHEN** the user presses the hotkey
- **THEN** the stop sound plays
- **AND** the sentence is pasted into the focused app
- **AND** listening stays on

### Requirement: Voice commands end a dictation the hotkey started
With voice control on, the system SHALL listen for the dictation phrases during a dictation the hotkey started, so "stop dictation" ends it and pastes its text without the command, and "cancel dictation" discards it.

#### Scenario: Saying stop dictation ends a hotkey dictation
- **GIVEN** voice control is turned on
- **AND** the user started a dictation with the hotkey and said a sentence
- **WHEN** the user says "stop dictation"
- **THEN** the sentence is pasted into the focused app without "stop dictation"

#### Scenario: Saying cancel dictation discards a hotkey dictation
- **GIVEN** voice control is turned on
- **AND** the user started a dictation with the hotkey and said a sentence
- **WHEN** the user says "cancel dictation"
- **THEN** nothing is pasted
- **AND** the cancel sound plays
- **AND** listening stays on

### Requirement: Listening stays on while voice control is on
With voice control on, the system SHALL start listening as soon as the app starts, without a hotkey press, and SHALL keep listening until the app exits or the command spotting service stops answering. The hotkey SHALL NOT turn listening off. Turning listening on SHALL play its sound.

#### Scenario: Listening is on at start
- **GIVEN** voice control is turned on
- **WHEN** the app starts
- **THEN** listening is on
- **AND** the listening-on sound plays

#### Scenario: Listening stays on after a hotkey dictation
- **GIVEN** listening is on
- **WHEN** the user records a dictation with the hotkey
- **THEN** listening is on
- **AND** saying "press enter" presses Return

### Requirement: Each command but cancel has a one-word short form
The system SHALL take "enter" as "press enter", "clear" as "clear all text", "start" as "start dictation" and "stop" as "stop dictation", with or without a trailing "please", under the same rules as the full phrase: said on its own, in the state that accepts the command. "cancel dictation" has no short form.

#### Scenario: A short form is recognized as its command
- **GIVEN** listening is on
- **WHEN** the user says "enter", "clear", "start" or "stop" on its own, in the state that accepts its command
- **THEN** "press enter", "clear all text", "start dictation" or "stop dictation" takes effect
