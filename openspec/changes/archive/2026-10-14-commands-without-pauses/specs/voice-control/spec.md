# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: Each command but cancel has a one-word short form
**Reason**: Replaced by "Every command has a one-word short form": the user asked for "cancel" alone, and "restart" is new.
**Migration**: None; the short forms of the other commands stay.

### Requirement: Each state accepts only its own commands
**Reason**: Replaced by "Each state accepts its own commands": "press enter" during a dictation now pastes and sends instead of being ignored, as the user asked.
**Migration**: None; to keep dictating, do not say "enter" on its own.

### Requirement: A command counts only when spoken on its own
**Reason**: Replaced by "A command counts on its own or in its set places": commands may now end an utterance, open it ("start"), or be doubled or chained without a pause.
**Migration**: None; a command said on its own counts as before.
