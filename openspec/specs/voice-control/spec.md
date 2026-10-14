# voice-control Specification

## Purpose
Lets the user dictate hands-free alongside push-to-talk: with voice control on, listening stays on while the app runs, the hotkey still records as push-to-talk, and six spoken English commands (each with a one-word short form) start, stop, cancel and restart a dictation, press Enter and clear the text in the focused app — said on their own, at the end of an utterance, doubled, or chained, with "start" also opening an utterance.

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
The system SHALL carry out a command once when the user repeats it within one utterance, with a pause shorter than the 0.3 s that ends an utterance ("press enter, press enter"). A longer pause separates two commands; a repeated "start", "stop" or "cancel" then has no effect, since the state it leads to does not accept it.

#### Scenario: A repeated press enter presses Return once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "press enter, press enter" without pausing
- **THEN** the Return key is pressed once

### Requirement: The user's own way of saying the commands is recognized
The system SHALL recognize the commands as the user says them, including small slips of pronunciation or of the speech recognition service ("star dictation", "stuck dictation", "presenter", "and restart" for a quick "enter start" closing a chain). Every command the user said in a recording kept for testing SHALL be recognized exactly once, apart from recordings marked unclear; repeats said without a pause of 0.3 s are one command. A short Russian phrase SHALL NOT be taken for a command; "стоп" is left out of this rule, since the speech recognition service may hear it as the short form "stop".

#### Scenario: Every recorded command of the user is recognized
- **GIVEN** the user's recordings of the commands, each labelled with the command said
- **WHEN** the recordings are heard one after another while listening, the app switching phrase sets as it does
- **THEN** each command said, repeats without a 0.3 s pause counted as one, is recognized exactly once
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

### Requirement: A command counts on its own or in its set places
The system SHALL carry out a command when its phrase is the whole utterance, with a pause before and after it, and in the places the requirements on commands said without a pause allow: at the end of an utterance, "start" at its start, a doubled short form or a chain anywhere. The same words inside a sentence — with speech after them in the same utterance, not doubled and not in a chain — SHALL NOT be taken as a command; a single short word right after a command ("press enter now") is within the spotter's timing error and may still count.

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

### Requirement: "press enter" during a dictation pastes and sends
During a dictation, the system SHALL take "press enter" or "enter", in any of its forms, as "stop dictation" followed by "press enter": it ends the dictation, pastes its text without the command, and then presses Return.

#### Scenario: Saying enter during a dictation pastes and sends
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "enter"
- **THEN** the sentence is pasted into the focused app
- **AND** the Return key is pressed after it

### Requirement: Each state accepts its own commands
The system SHALL accept "start dictation" and "clear all text" only while listening with no dictation running, "stop dictation", "cancel dictation" and "restart dictation" only during a dictation, and "press enter" in both. A command spoken in a state that does not accept it SHALL have no effect.

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

### Requirement: Every command has a one-word short form
The system SHALL take "start" as "start dictation", "stop" as "stop dictation", "cancel" as "cancel dictation", "restart" as "restart dictation", "enter" as "press enter" and "clear" as "clear all text", with or without a trailing "please", under the same rules as the full phrase, in the state that accepts the command.

#### Scenario: A short form is recognized as its command
- **GIVEN** listening is on
- **WHEN** the user says "start", "stop", "cancel", "restart", "enter" or "clear" on its own, in the state that accepts its command
- **THEN** "start dictation", "stop dictation", "cancel dictation", "restart dictation", "press enter" or "clear all text" takes effect

#### Scenario: Cancel alone discards a dictation
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "cancel"
- **THEN** no text is pasted

### Requirement: "restart dictation" starts the dictation over
During a dictation, the system SHALL discard what was dictated so far, paste nothing, and start a new dictation at once when the user says "restart dictation" or its short form "restart", with or without "please". It plays the cancel sound and then the start sound. "restart restart" counts anywhere, like the other doubled short forms, and "restart" may end an utterance. It is in no chain: it only follows a running dictation, and nothing follows it in one breath.

#### Scenario: Saying restart drops the dictation and starts a new one
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "restart"
- **THEN** nothing is pasted
- **AND** the cancel sound plays, then the start sound
- **AND** a new dictation starts

#### Scenario: Only the speech after a restart is pasted
- **GIVEN** the user said a sentence, "restart", and another sentence
- **WHEN** the user says "stop"
- **THEN** only the other sentence is pasted

#### Scenario: Restart restart in the middle of speech
- **GIVEN** a dictation runs
- **WHEN** someone says a sentence and "restart restart" without a pause
- **THEN** the dictation starts over

### Requirement: A command may end an utterance
The system SHALL carry out a command whose phrase (full or short, with or without "please") ends an utterance, even with speech right before it in the same utterance, when the state accepts the command. The utterance ends with a pause of at least 0.3 s after the command. When the command ends a dictation, the pasted text SHALL hold the speech before the command and SHALL NOT hold the command words.

#### Scenario: Stop right after the last dictated word
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence and then "stop" without a pause, then pauses
- **THEN** the sentence is pasted into the focused app without "stop"

#### Scenario: Enter right after other speech
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "okay, press enter" without a pause, then pauses
- **THEN** the Return key is pressed in the focused app

#### Scenario: A command word inside a sentence is still no command
- **GIVEN** a dictation runs
- **WHEN** the user says "we should stop the server tonight" and pauses
- **THEN** the dictation keeps running

### Requirement: "start" may open an utterance
The system SHALL start a dictation when "start dictation" or "start" opens an utterance and speech follows it without a pause, while no dictation runs; the speech after the command SHALL be part of the dictation and the command words SHALL NOT be pasted. Other commands that open an utterance followed by speech SHALL have no effect, and so SHALL "start" followed at once by another command word ("start stop").

#### Scenario: Start followed by the dictation at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start" and a sentence without a pause, and later ends the dictation
- **THEN** the sentence is pasted into the focused app without "start"

#### Scenario: Enter followed by speech is no command
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter the building on the left" and pauses
- **THEN** no key is pressed

### Requirement: A doubled short form counts anywhere
The system SHALL carry out a command at once when its short form is said twice in a row — "start start", "stop stop", "cancel cancel", "enter enter", "clear clear" — anywhere in an utterance, with no pause around it, in a state that accepts the command. When "stop stop" or "cancel cancel" ends a dictation, the words after it in the same utterance SHALL NOT be pasted; with "stop stop" the words before it SHALL be pasted.

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

### Requirement: A chain of commands is carried out in order
The system SHALL carry out a chain of two or three different commands said one right after the other — full phrases or short forms — in the order said, when each command is accepted in the state the one before it leads to and "start" comes last. A chain SHALL count anywhere in an utterance, with no pause around it.

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

### Requirement: Which chains exist
Since "start dictation" leads to dictating and "stop dictation" or "cancel dictation" lead back to standby, the chains SHALL be exactly those where "start" comes last: during a dictation, "stop" or "cancel" followed by one or two of "enter", "clear", "start" (each once, "start" last), or "enter" followed by one or two of "clear", "start" ("enter" ends the dictation and pastes first); with no dictation, two or three of "enter", "clear", "start" (each once, "start" last).

#### Scenario: Stop clear enter
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop clear enter"
- **THEN** the sentence is pasted, then select all and delete are pressed, then the Return key is pressed

### Requirement: An utterance ends after 0.3 s of silence
The system SHALL end an utterance after 0.3 s of silence, so a command said after a short breath is an utterance of its own.

#### Scenario: Stop after a short breath
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence, pauses for about 0.35 s and says "stop dictation", then pauses
- **THEN** the sentence is pasted into the focused app without "stop dictation"

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
