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
During a dictation, the system SHALL end the dictation when the user says "stop dictation", then transcribe and paste the text as a push-to-talk recording does. The spoken words "stop dictation" SHALL NOT appear in the pasted text. Listening SHALL stay on afterwards.

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
While listening and no dictation runs, the system SHALL press the Return key in the focused app when the user says "press enter". It SHALL then play the enter sound and show the enter sign on the overlay.

#### Scenario: Saying press enter presses Return
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "press enter"
- **THEN** the Return key is pressed in the focused app
- **AND** the enter sound plays
- **AND** the overlay shows the enter sign for about a second, then hides

### Requirement: "clear all text" erases the focused text
While listening and no dictation runs, the system SHALL select all and delete in the focused app when the user says "clear all text": it presses Cmd+A, then Delete. It SHALL then play the clear sound and show the clear sign on the overlay.

#### Scenario: Saying clear all text selects all and deletes
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "clear all text"
- **THEN** Cmd+A and then Delete are pressed in the focused app
- **AND** the clear sound plays
- **AND** the overlay shows the clear sign for about a second, then hides

### Requirement: Commands are carried out in the order spoken
The system SHALL carry out commands in the order they were spoken. A command spoken while the previous dictation is still being transcribed or pasted SHALL wait until that text is pasted.

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
With voice control on, the system SHALL start listening as soon as the app starts, without a hotkey press, and SHALL keep listening until the app exits; while the command spotting service is away, listening SHALL resume on its own once the service answers again. The hotkey SHALL NOT turn listening off. Turning listening on SHALL play its sound.

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
During a dictation, the system SHALL take "press enter" or "enter", in any of its forms, as "stop dictation" followed by "press enter": it ends the dictation, pastes its text without the command, and then presses Return, with the enter sound and sign once Return is pressed.

#### Scenario: Saying enter during a dictation pastes and sends
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "enter"
- **THEN** the sentence is pasted into the focused app
- **AND** the Return key is pressed after it
- **AND** the enter sound plays and the overlay shows the enter sign after the paste

### Requirement: Each state accepts its own commands
The system SHALL accept "start dictation", "clear all text" and "paste text" only while listening with no dictation running, "stop dictation", "cancel dictation" and "restart dictation" only during a dictation, and "press enter" in both. A command spoken in a state that does not accept it SHALL have no effect. "start" said during a dictation — on its own, ending a sentence ("let's start") or inside one, at a normal level or in a quiet voice — SHALL neither be taken for "restart" nor end the dictation.

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

#### Scenario: Paste during a dictation is ignored
- **GIVEN** a dictation runs
- **WHEN** the user says "paste text"
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

### Requirement: Every command has a one-word short form
The system SHALL take "start" as "start dictation", "stop" as "stop dictation", "cancel" as "cancel dictation", "restart" as "restart dictation", "enter" as "press enter", "clear" as "clear all text" and "paste" as "paste text", with or without a trailing "please", under the same rules as the full phrase, in the state that accepts the command.

#### Scenario: A short form is recognized as its command
- **GIVEN** listening is on
- **WHEN** the user says "start", "stop", "cancel", "restart", "enter", "clear" or "paste" on its own, in the state that accepts its command
- **THEN** "start dictation", "stop dictation", "cancel dictation", "restart dictation", "press enter", "clear all text" or "paste text" takes effect

#### Scenario: Cancel alone discards a dictation
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "cancel"
- **THEN** no text is pasted

### Requirement: "restart dictation" starts the dictation over
During a dictation, the system SHALL discard what was dictated so far, paste nothing, and start a new dictation at once when the user says "restart dictation" or its short form "restart", with or without "please". It plays the restart sound alone, neither the cancel sound nor the start sound, and the overlay shows the restart sign for about a second, then the recording again. "restart restart" counts anywhere, like the other doubled short forms, and "restart" may end an utterance. It is in no chain: it only follows a running dictation, and nothing follows it in one breath.

#### Scenario: Saying restart drops the dictation and starts a new one
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "restart"
- **THEN** nothing is pasted
- **AND** the restart sound plays, and neither the cancel sound nor the start sound
- **AND** the overlay shows the restart sign, then the recording
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
The system SHALL carry out a command at once when its short form is said twice in a row — "start start", "stop stop", "cancel cancel", "enter enter", "clear clear", "paste paste" — anywhere in an utterance, with no pause around it, in a state that accepts the command. When "stop stop" or "cancel cancel" ends a dictation, the words after it in the same utterance SHALL NOT be pasted; with "stop stop" the words before it SHALL be pasted.

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

#### Scenario: The end of a chain heard only when the utterance ends still counts
- **GIVEN** a dictation runs
- **AND** while the user speaks, the system has heard only the first two commands of "never mind cancel enter start" and carried them out
- **WHEN** the utterance ends and the system hears all of it
- **THEN** "start dictation" is carried out after them
- **AND** "cancel dictation" and "press enter" are not carried out again

### Requirement: Which chains exist
Since "start dictation" leads to dictating and "stop dictation" or "cancel dictation" lead back to standby, the chains SHALL be exactly those where "start" comes last: during a dictation, "stop" or "cancel" followed by one or two of "enter", "clear", "paste", "start" (each once, "start" last), or "enter" followed by one or two of "clear", "paste", "start" ("enter" ends the dictation and pastes first); with no dictation, two or three of "enter", "clear", "paste", "start" (each once, "start" last).

#### Scenario: Stop clear enter
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop clear enter"
- **THEN** the sentence is pasted, then select all and delete are pressed, then the Return key is pressed

#### Scenario: Paste enter pastes the clipboard and sends
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "paste enter"
- **THEN** Cmd+V is pressed, then the Return key is pressed

### Requirement: An utterance ends after 0.3 s of silence
The system SHALL end an utterance after 0.3 s of silence, so a command said after a short breath is an utterance of its own, also in a noisy room where the silence is the room's own noise. This SHALL hold for a quiet voice and when speech follows the pause at once: a command said on its own, followed by 0.3–0.6 s of silence and then speech, SHALL take effect. The speech after the pause SHALL NOT change what the command is heard as. A pause is the silence between the words as said, not the silence a speech synthesizer adds after a word.

#### Scenario: Stop after a short breath
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence, pauses for about 0.35 s and says "stop dictation", then pauses
- **THEN** the sentence is pasted into the focused app without "stop dictation"

#### Scenario: Restart, a short pause, then speech that starts like another word
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "restart", pauses for about 0.4 s and says another sentence that opens with a hard consonant, and later ends the dictation
- **THEN** only the other sentence is pasted, its first words included

#### Scenario: A command alone, a short pause, then speech
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter", pauses for about 0.3 s and says a sentence, then pauses
- **THEN** the Return key is pressed once
- **AND** no other command takes effect

#### Scenario: The user's recorded command alone, a short pause, then speech
- **GIVEN** the user's own recordings of the commands exist on the machine
- **AND** the state accepts the command
- **WHEN** the user's recorded "restart", "start", "stop" or "enter", said on its own, is followed by 0.3–0.6 s of silence and then speech
- **THEN** the command takes effect once
- **AND** no other command takes effect

#### Scenario: A command in a noisy room ends at its pause
- **GIVEN** listening is on
- **AND** no dictation runs
- **AND** steady room noise plays
- **WHEN** the user says "start" and stays silent
- **THEN** the start cue comes within 0.7 s of the end of "start"

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

### Requirement: Voice control needs the command spotting service to start
With voice control on and the services left to the service scripts, the system SHALL refuse to start, with a message naming the command spotting service and how to start it, when that service does not answer. When the app manages its services, it starts the command spotting service instead and waits for it.

#### Scenario: The app refuses to start without the command spotting service
- **GIVEN** voice control is turned on
- **AND** the app leaves its services to the service scripts
- **AND** the command spotting service is not running
- **WHEN** the app starts
- **THEN** the app exits with an error that names the command spotting service and how to start it

### Requirement: The app keeps reaching for the command spotting service while it is away
When the command spotting service stops answering while listening, the system SHALL log an error naming the service and keep trying to reach the service, with a growing pause of at most 4 seconds, until it answers or the app exits; it SHALL log the first failed attempt and every tenth, so a long outage does not flood the log. The loss SHALL play no sound, since listening stays on.

#### Scenario: The app keeps reaching for the command spotting service when it goes away
- **GIVEN** listening is on
- **WHEN** the command spotting service stops answering
- **THEN** an error naming the command spotting service is logged
- **AND** no sound plays
- **AND** the app tries to reach the service again, logging its first failed attempt and every tenth

### Requirement: Listening comes back when the command spotting service is back
When the command spotting service answers again after it went away, the system SHALL resume listening for the commands of the current state within 5 seconds and log that listening is back, without a restart of the app and without playing a sound. The system SHALL NOT give up reaching for the service however many attempts fail: an outage that outlasts the speech it replays and the tenth failed attempt still ends with listening back.

#### Scenario: Commands work again after the command spotting service restarts
- **GIVEN** listening is on and no dictation runs
- **AND** the command spotting service went away and was started again
- **WHEN** the user says "press enter" 5 seconds after the service answers again
- **THEN** the Return key is pressed in the focused app
- **AND** no listening sound played after the app started

#### Scenario: Listening is not given up after many failed attempts
- **GIVEN** listening is on
- **AND** the command spotting service stays away past the speech the app replays and past the tenth failed attempt to reach it
- **WHEN** the service answers again
- **THEN** listening is on again within 5 seconds
- **AND** the first failed attempt and the tenth were logged, and the service was reported still away once

### Requirement: A slow command spotting service does not stop listening
The system SHALL keep listening while the command spotting service answers late, however long its spotting results take, as long as the service still answers on its connection; only a dropped connection or a service that answers nothing at all for 5 seconds after the app asked SHALL count as the service going away. A pause of the app's own, during which it reads nothing from the service, SHALL NOT count as the service's silence.

#### Scenario: A confirmation that takes longer than 10 seconds keeps listening on
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands holds one confirmation of the command spotting service for longer than 10 seconds, past the time the service waits for a confirmation
- **WHEN** the user keeps dictating through that hold and then says "stop dictation"
- **THEN** listening is still on and no error about the command spotting service is logged
- **AND** the dictation ends and its text is pasted

#### Scenario: A pause of the app's own does not lose the service
- **GIVEN** listening is on and the command spotting service answers every check at once
- **WHEN** the app reads nothing from the service for 6 seconds, as when it is busy or the computer sleeps
- **THEN** the service is not reported lost and no error about the command spotting service is logged

#### Scenario: A burst of audio after long listening does not lose the service
- **GIVEN** listening is on with a freshly started command spotting service and the machine is busy
- **WHEN** 30 seconds of silence followed by "start dictation" reach the app at once
- **THEN** the service is not reported lost
- **AND** "start dictation" is recognized

### Requirement: A dictation keeps recording while the command spotting service is away
The system SHALL keep a running dictation recording when the command spotting service goes away; the hotkey SHALL still end it, and once the service is back the dictation's voice commands SHALL end it as usual.

#### Scenario: The hotkey ends a dictation while the service is away
- **GIVEN** a dictation started by voice runs
- **AND** the command spotting service went away during it
- **WHEN** the user presses the hotkey to stop
- **THEN** the dictation ends and its text is pasted

#### Scenario: "stop dictation" ends a dictation after the service is back
- **GIVEN** a dictation started by voice runs
- **AND** the command spotting service went away during it and is back
- **WHEN** the user says "stop dictation"
- **THEN** the dictation ends and its text is pasted

### Requirement: A command said while the command spotting service was away still counts
The system SHALL send the service the speech heard during the last 10 seconds of an outage once the service is back, so a command said in that time is carried out late rather than lost. Microphone audio that never reached the app before the outage SHALL NOT shift the speech sent again: it still begins before the moment the service went away.

#### Scenario: "stop dictation" said during a short outage ends the dictation
- **GIVEN** a dictation runs
- **AND** the command spotting service is away for 3 seconds
- **WHEN** the user says "stop dictation" during those 3 seconds
- **THEN** the dictation ends once the service is back, and the words "stop dictation" are not in the pasted text

#### Scenario: Speech sent again after an outage begins before the loss when microphone audio was lost earlier
- **GIVEN** listening is on and some seconds of microphone audio never reached the app
- **AND** the command spotting service went away afterwards
- **WHEN** the service is back
- **THEN** the speech sent to it again begins before the moment the service went away

### Requirement: Confirmations are not steered by a prompt
The command spotting service SHALL send every confirmation to the speech recognition service without a prompt, so speech that is no command is not written as a command phrase. Measured with the command phrases as the prompt, Russian counting words and short Russian phrases were confirmed as commands and the user's recorded speech gave more wrong commands than without it.

#### Scenario: A confirmation carries no prompt
- **GIVEN** voice control is listening
- **WHEN** an utterance is sent for confirmation
- **THEN** the request to the speech recognition service holds no prompt

#### Scenario: Speech that is no command is not turned into a command
- **GIVEN** voice control is listening
- **WHEN** someone says short Russian phrases ("да", "спасибо", "привет") or Russian counting words, each on its own with a pause around it
- **THEN** no command takes effect

### Requirement: Every confirmed command sounds different
The enter, clear, paste and restart sounds SHALL differ from each other and from the start, stop, cancel, listening-on and listening-off sounds, unless the user picks the same sound for two of them. Each of the four SHALL be the user's to change, like the other sounds, with a distinct sound by default.

#### Scenario: Enter and clear sound different by default
- **GIVEN** the user has not changed any sound
- **WHEN** the user says "enter", and later "clear"
- **THEN** the two commands play different sounds

#### Scenario: Paste sounds different from enter and clear by default
- **GIVEN** the user has not changed any sound
- **WHEN** the user says "paste", and later "enter" and "clear"
- **THEN** the paste sound differs from the enter and clear sounds

#### Scenario: A sound the user picked for a command plays
- **GIVEN** the user picked another sound for "clear all text"
- **WHEN** the user says "clear all text"
- **THEN** the picked sound plays

### Requirement: A confirmation sign gives way to what comes next
The enter, clear, paste and restart signs SHALL each name the command that ran. A sign SHALL give way at once to what the overlay shows next — the recording of a dictation a chain starts, or the sign of the next command in the chain — and after about a second the overlay SHALL return to what it showed before the command: hidden in standby, the recording during a dictation.

#### Scenario: Enter start shows the recording right after the enter sign
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter start"
- **THEN** the enter sound plays and the overlay shows the enter sign
- **AND** the overlay shows the recording as soon as the new dictation starts

#### Scenario: Stop clear enter ends on the enter sign
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop clear enter"
- **THEN** the stop sound, the clear sound and the enter sound play in this order
- **AND** the overlay shows the clear sign, then the enter sign, then hides

### Requirement: Push-to-talk keeps its sounds and overlay
A recording started and ended by the hotkey SHALL play the sounds and show the overlay it shows today — the recording, then processing, then the Pasted sign — with no command sign; Escape during it SHALL play the cancel sound and hide the overlay.

#### Scenario: A hotkey dictation shows no command sign
- **GIVEN** listening is on
- **WHEN** the user records a sentence with the hotkey
- **THEN** the start and stop sounds play
- **AND** the overlay shows the recording, then processing, then the Pasted sign, then hides, with no command sign

### Requirement: A voice start gives its start cue within 0.7 s
When a voice command starts a dictation from standby, or "restart" starts one over, the system SHALL play the sound that start plays (the restart sound for "restart") and show the overlay's recording sign as soon as the command is confirmed, without waiting for the microphone of the dictation to open, within 0.7 s of the end of the command's words in at least 9 of 10 starts, with the services otherwise idle; a doubled form ("start start", "restart restart"), which is heard as a command without waiting for the pause after it or the next word, SHALL give it within 0.5 s. In the scenarios below "the start cue" means that sound. This holds for "start" alone, opening or ending an utterance, or doubled, with "please" after it, and every form of "restart", also when the first hearing of a quiet "restart" takes it for another word. A check of this requirement SHALL time the start cue only on an idle machine: it waits for the builds, tests and model stages of every other check on the machine to end and keeps new ones out while it times, so the bound is measured under the condition it states. A start cue over its bound SHALL be reported with what ran beside it: the busiest processes on the machine and every check that held a lock at that moment. A check SHALL time the start cue only against warm copies of the speech recognition service for commands: each copy has transcribed speech since it started, so no confirmation of a timed start waits for a copy's first decode. A start cue that never came because the confirmation of the command was cut short SHALL be reported with the copies whose confirmation was cut short, after how long each was cut short, and what ran beside them, as a late cue is.

#### Scenario: Start start followed by the dictation at once gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and a sentence without a pause
- **THEN** the start cue comes within 0.5 s of the end of "start start", while the sentence is still being said

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
- **THEN** the start cue of the new dictation comes within 0.5 s of the end of "restart restart"

#### Scenario: Restart said on its own gives its cue at once
- **GIVEN** a dictation runs
- **AND** the user has dictated for about 20 s
- **WHEN** the user says "restart" and stays silent
- **THEN** the start cue of the new dictation comes within 0.7 s of the end of "restart"

#### Scenario: The user's recorded starts give their cue at once
- **GIVEN** the user's own recordings of the commands exist on the machine
- **AND** listening is on
- **WHEN** each of the user's recorded starts is played, on its own and followed at once by a sentence
- **THEN** at least 9 of 10 of them give the start cue within its bound of the end of the command's words

#### Scenario: The overlay shows with the start cue
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and stays silent
- **THEN** the overlay's recording sign shows within 0.05 s of the start cue

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

#### Scenario: Start please followed by the dictation at once gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start please" and a sentence without a pause
- **THEN** the start cue comes within 0.7 s of the end of "start please", while the sentence is still being said

#### Scenario: Start please said on its own gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start please" and stays silent
- **THEN** the start cue comes within 0.7 s of the end of "start please"

#### Scenario: A brief dip inside the command words starts nothing early
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start" and "stop" with a gap between them shorter than a pause
- **THEN** no dictation starts

#### Scenario: The start cue is timed once the other checks' builds have ended
- **GIVEN** another check on the machine is building or testing while this check reaches its cue-timing scenarios
- **WHEN** the cue-timing scenarios are due
- **THEN** they wait for that build or test to end, no new build or test of another check starts beside them, and they time the start cue only then

#### Scenario: A late start cue names what ran beside it
- **GIVEN** the start cue of a voice start came later than its bound
- **WHEN** the failure is reported
- **THEN** the report names the busiest processes on the machine and every check that held a lock at that moment

#### Scenario: The start cue is timed against warm copies
- **GIVEN** the speech recognition service for commands was started for a check right before its cue-timing scenarios
- **WHEN** the first cue-timing scenario confirms its command
- **THEN** every copy of that service has already transcribed speech, and the confirmation does not wait for a copy's first decode

#### Scenario: A missing start cue names the confirmations cut short
- **GIVEN** the start cue of a voice start never came because the confirmation of its command was cut short on every copy
- **WHEN** the failure is reported
- **THEN** the report names each copy whose confirmation was cut short and after how long it was cut short, beside the busiest processes on the machine and every check that held a lock at that moment

### Requirement: Every voice start logs how long its start cue took
For every voice start in scope of the start cue bound, the system SHALL log one line with the time from the end of the command's words to the start cue and the time of each stage on the way: hearing the command, confirming it (or that it was not confirmed), delivering it to the app, starting the cue sound and drawing the overlay. The time SHALL stay right after part of the microphone audio never reached the command spotting service.

#### Scenario: A voice start logs its timing
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and a sentence without a pause
- **THEN** the log shows one timing line for that start, with its total and the time of each stage, the cue sound and the overlay included

#### Scenario: A start without confirmation logs that it was not confirmed
- **GIVEN** the confirmation of commands is off
- **AND** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and stays silent
- **THEN** the timing line of that start shows that it was not confirmed

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

### Requirement: The overlay shows that voice commands do not work while the service stays away
When the command spotting service has been away for 5 seconds while listening, the system SHALL show on the overlay that voice commands do not work now, until the service is back: alone in standby, beside the recording or processing sign during a dictation. The sign SHALL NOT take focus. Once the service is back, the overlay SHALL show only what the current state shows.

#### Scenario: A short outage shows nothing
- **GIVEN** listening is on and no dictation runs
- **WHEN** the command spotting service goes away and is back within 5 seconds
- **THEN** the overlay never shows that voice commands do not work
- **AND** no sound plays

#### Scenario: A long outage in standby shows the sign until the service is back
- **GIVEN** listening is on, no dictation runs and the overlay is hidden
- **WHEN** the command spotting service stays away for longer than 5 seconds
- **THEN** the overlay appears and shows that voice commands do not work
- **AND** once the service is back, the overlay hides

#### Scenario: A long outage during a dictation shows the sign beside the recording sign
- **GIVEN** a dictation runs and the overlay shows the recording sign
- **WHEN** the command spotting service stays away for longer than 5 seconds
- **THEN** the overlay shows that voice commands do not work beside the recording sign
- **AND** once the service is back, the overlay shows the recording sign alone

#### Scenario: A dictation that ends during a long outage keeps the sign
- **GIVEN** a dictation runs and the overlay shows that voice commands do not work
- **WHEN** the user ends the dictation with the hotkey and its text is pasted
- **THEN** the overlay still shows that voice commands do not work
- **AND** once the service is back, the overlay hides

### Requirement: A command is carried out once however fast its confirmation comes back
The command spotting service SHALL carry out every command said exactly once, as the command it was said as, whether the speech recognition service for commands answers its confirmations at once or slowly within its time limit; a check that falls due while another is still waiting SHALL wait its turn and hear the same audio it would have heard on time, never be skipped. A confirmation SHALL be read against the commands that were listened for when its words were heard together with those listened for when it comes back, the closest command winning, so a switch to the commands of another state before the confirmation comes back never turns the command said into another one; a command the app's current state does not accept is then ignored, as any command said in the wrong state is. Which commands are carried out SHALL depend only on what was said and on how late each confirmation comes back measured against the speech, never on how busy the machine is while it listens.

#### Scenario: A command is carried out once while confirmations come back slowly
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 2 seconds to answer each confirmation
- **WHEN** the user says the doubled short form "restart restart" in the middle of dictated speech
- **THEN** the dictation is restarted exactly once

#### Scenario: A chain is carried out the same at any confirmation speed
- **GIVEN** listening is on and a dictation runs
- **WHEN** the user says "stop enter start" once while confirmations come back at once and once while each takes 2 seconds
- **THEN** both times the dictation ends, its text is pasted, Return is pressed and a new dictation starts, each exactly once

#### Scenario: A start confirmed after the dictation began stays a start
- **GIVEN** listening is on and no dictation runs
- **AND** the speech recognition service for commands takes 2 seconds to answer each confirmation
- **AND** a dictation is started by the hotkey right after the user said "start dictation", before its confirmation comes back
- **WHEN** the confirmation of "start dictation" comes back
- **THEN** it is taken as "start dictation"
- **AND** the dictation is not restarted

#### Scenario: A start said right after a slowly confirmed stop stays a start
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 2.5 seconds to answer each confirmation
- **WHEN** the user says "stop dictation" and then, before its confirmation comes back, "start"
- **THEN** the dictation ends and a new dictation starts

#### Scenario: A chain is carried out the same on a busy machine
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 2 seconds to answer each confirmation
- **AND** the machine is busy with other work, so the speech is taken in late and in bursts
- **WHEN** the user says "stop enter start"
- **THEN** the dictation ends, its text is pasted, Return is pressed and a new dictation starts, each exactly once, as on an idle machine

### Requirement: "paste text" pastes the clipboard
While listening and no dictation runs, the system SHALL press Cmd+V in the focused app when the user says "paste text", so the focused app pastes whatever the clipboard holds at its cursor. The command SHALL leave the clipboard as it is. It SHALL then play the paste sound and show the paste sign on the overlay. With an empty clipboard the focused app pastes nothing, and the command still confirms itself.

#### Scenario: Saying paste text pastes the clipboard at the cursor
- **GIVEN** listening is on
- **AND** no dictation runs
- **AND** the clipboard holds a sentence
- **WHEN** the user says "paste text"
- **THEN** Cmd+V is pressed in the focused app
- **AND** the paste sound plays
- **AND** the overlay shows the paste sign for about a second, then hides

#### Scenario: Paste leaves the clipboard as it was
- **GIVEN** listening is on
- **AND** no dictation runs
- **AND** the clipboard holds a sentence
- **WHEN** the user says "paste"
- **THEN** the clipboard still holds the same sentence

#### Scenario: A paste phrase inside a sentence is ignored
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** someone says "I will paste the link later" without pausing
- **THEN** no key is pressed

### Requirement: A confirmation never waits past its deadline for a copy
The command spotting service SHALL send a confirmation to a free copy of the speech recognition service for commands, or wait for one only until the confirmation's deadline; a confirmation that finds no free copy by then SHALL be given up and logged without ever being sent, so given-up confirmations never occupy a copy.

#### Scenario: Every copy for commands is busy past the deadline
- **GIVEN** voice control runs, and every copy of the speech recognition service for commands is busy for longer than a confirmation's deadline
- **WHEN** the user says "press enter"
- **THEN** that confirmation is logged as given up and ends without waiting much past its deadline, and no copy receives it

#### Scenario: A copy frees before the deadline
- **GIVEN** voice control runs, and every copy of the speech recognition service for commands is busy for one more second
- **WHEN** the user says "press enter"
- **THEN** the confirmation is sent to the first copy that frees, and "press enter" is reported

### Requirement: Listening moves to another copy of the command spotting service
When the copy of the command spotting service the app streams to goes away while another copy answers, the system SHALL connect to that other copy at its first attempt to reach the service again and replay the speech the lost copy may have missed, so a command said meanwhile still counts and the overlay never shows that voice commands do not work.

#### Scenario: The command spotting copy the app listens with is killed
- **GIVEN** the app started the command spotting service as two copies and listening is on
- **WHEN** the copy the app streams to is killed and the user says "press enter" a moment later
- **THEN** the Return key is pressed in the focused app
- **AND** the overlay did not show that voice commands do not work
- **AND** the killed copy answers again within 30 seconds

### Requirement: A confirmation refused by its copy goes to another copy
The command spotting service SHALL send a confirmation whose copy of the speech recognition service for commands refused the connection to another copy that is up, within the confirmation's deadline, and SHALL log the refused attempt.

#### Scenario: A copy for commands is down as a confirmation reaches it
- **GIVEN** the speech recognition service for commands runs as two copies, and one stopped accepting connections a moment ago
- **WHEN** a confirmation is sent to that copy
- **THEN** the other copy confirms it, and the refused attempt is logged

#### Scenario: A copy for commands is killed while voice control runs
- **GIVEN** the app started the speech recognition service for commands as its copies and listening is on
- **WHEN** one copy is killed and the user says "press enter" right after
- **THEN** the Return key is pressed in the focused app

### Requirement: A confirmation whose copy stalls goes to another copy
The command spotting service SHALL send a confirmation whose copy of the speech recognition service for commands does not answer within its decoding budget to another copy while the confirmation's deadline lasts, and SHALL never take a copy's silence for speech without a command.

#### Scenario: One copy for commands stalls mid-command
- **GIVEN** the speech recognition service for commands runs as two copies and one stops answering
- **WHEN** a confirmation of "press enter" said alone reaches the stalled copy
- **THEN** the other copy confirms it within the confirmation's deadline and "press enter" is reported

#### Scenario: Every copy for commands stalls
- **GIVEN** every copy of the speech recognition service for commands stops answering
- **WHEN** a confirmation of "press enter" said alone is asked
- **THEN** it ends as given up, not as speech heard without a command

### Requirement: A command whose confirmation fails is reported
The command spotting service SHALL report a closed speech segment whose confirmation got no answer from the speech recognition service for commands as a failed confirmation, and the system SHALL then show that voice commands are unavailable, within 10 seconds of the segment's end.

#### Scenario: The service reports a segment it could not confirm
- **GIVEN** every copy of the speech recognition service for commands stops answering
- **WHEN** an app streams "press enter" said alone
- **THEN** the service reports that the confirmation of that segment failed, within 10 seconds of its end

#### Scenario: The app shows a failed confirmation
- **GIVEN** listening is on
- **WHEN** the command spotting service reports a failed confirmation
- **THEN** the overlay shows the failure notice that voice commands are unavailable

### Requirement: Audio the command spotting service cannot keep up with is dropped and reported
The command spotting service SHALL let no audio of a connection wait for processing longer than 10 seconds; audio that arrives while the oldest waiting audio has waited longer SHALL be dropped, while audio sent faster than real time that the service keeps up with (a replay) SHALL be kept, and reported to the app with the number of samples dropped, and the system SHALL then count the stream as lost and connect again, replaying the speech the service may have missed.

#### Scenario: A flood of audio past the backlog is dropped
- **GIVEN** the command spotting service processes audio slower than it arrives
- **WHEN** audio arrives while the oldest waiting audio has waited more than 10 seconds
- **THEN** that audio is dropped and the app is told how many samples were dropped

#### Scenario: Audio sent faster than real time is kept while the service keeps up
- **GIVEN** the command spotting service keeps up with its processing
- **WHEN** 30 seconds of audio arrive at once, as a replay sends them
- **THEN** none of it is dropped

#### Scenario: The app connects again after audio was dropped
- **GIVEN** listening is on
- **WHEN** the command spotting service reports dropped audio
- **THEN** the service is counted as lost and the app reaches for it again

### Requirement: The app and the command spotting service agree on a protocol version
Every connection to the command spotting service SHALL open with the protocol version the app speaks; the service SHALL close a connection whose version differs, or whose first message carries none, with a reason naming both versions, and SHALL close a connection that sends a malformed message or an audio frame that is not whole samples with a reason saying what was wrong.

#### Scenario: A client of another protocol version is refused
- **GIVEN** the command spotting service runs
- **WHEN** a client opens a connection with another protocol version
- **THEN** the service closes it with a reason naming the version it speaks

#### Scenario: A malformed message closes the connection with a reason
- **GIVEN** a client is connected with the right protocol version
- **WHEN** it sends a message that is not valid
- **THEN** the service closes the connection with a reason saying the message could not be read

#### Scenario: An audio frame of partial samples closes the connection with a reason
- **GIVEN** a client is connected with the right protocol version
- **WHEN** it sends an audio frame whose length is not a whole number of samples
- **THEN** the service closes the connection with a reason saying so

### Requirement: Every check due for a spotted "start" is asked
When the first check of a spotted "start" or "restart" finds no command while the check of another spotted phrase still waits its turn, the command spotting service SHALL keep both checks and ask each when it falls due.

#### Scenario: A second check does not replace a check still waiting
- **GIVEN** a check of a spotted "start" waits its turn
- **WHEN** the first check of an earlier spotted "start" finds no command
- **THEN** both the waiting check and the second check of the earlier phrase are asked

### Requirement: A command confirmed slowly under load is still carried out
When the speech recognition service for commands answers its confirmations slowly because the machine is busy, the command spotting service SHALL still report a command said alone or in its place, in the order spoken, within the confirmation's deadline, instead of dropping it as speech without a command.

#### Scenario: A stop right after dictated speech under load
- **GIVEN** voice control runs, a dictation is on, and the speech recognition service for commands takes over a second for every confirmation, as under machine load
- **WHEN** the user dictates a sentence and says "stop dictation" right after its last word
- **THEN** the command spotting service reports "stop dictation"

#### Scenario: A chain of commands under load
- **GIVEN** voice control runs and the speech recognition service for commands takes over a second for every confirmation, as under machine load
- **WHEN** the user says "that is all stop enter start"
- **THEN** the command spotting service reports "stop dictation", "press enter" and "start dictation", in that order

#### Scenario: A restart before a short pause and more speech under load
- **GIVEN** voice control runs, a dictation is on, and the speech recognition service for commands takes over a second for every confirmation, as under machine load
- **WHEN** the user says "restart", pauses for about 0.4 s and says another sentence
- **THEN** the command spotting service reports "restart dictation"

### Requirement: Every event of audio accepted before a sync is out before the sync is answered
When an app asks the command spotting service to confirm that every event of the audio it sent so far is out (a sync), the service SHALL answer only after it has reported every speech segment closed in that audio, also a segment whose confirmation it had to ask again — because the first hearing came back empty, cut short under load or misheard — after the sync arrived; an app that closes the connection on that answer SHALL have received every command said in that audio.

#### Scenario: A command whose first hearing is empty under load is reported before the sync is answered
- **GIVEN** voice control runs in standby and the speech recognition service for commands answers the first hearing of a segment empty and late, as under machine load, and the next one right
- **WHEN** the user says "clear text please" alone and the app asks for a sync right after the speech
- **THEN** the command spotting service reports "clear all text" before it answers the sync

#### Scenario: A command whose half-pause hearing is misheard is reported before the sync is answered
- **GIVEN** voice control runs in standby and the speech recognition service for commands mishears a command word in the hearing asked at half the pause and hears it right in the hearing of the whole segment
- **WHEN** the user says "okay, clear" with a short pause after "okay" and the app asks for a sync right after the speech
- **THEN** the command spotting service reports "clear all text" before it answers the sync

### Requirement: Listening comes back when the microphone stalls
While listening, when the microphone used for voice commands reports an error or delivers no audio for 2 seconds (5 seconds before its first audio), the system SHALL log it once and open the same input device again, retrying every second until it opens, then go on listening for the same commands without an app restart. The silent time SHALL be streamed as silence, so later audio keeps its capture times.

#### Scenario: A microphone that stops delivering audio is opened again
- **GIVEN** listening is on and the microphone delivered audio
- **WHEN** the microphone stops delivering audio without reporting an error
- **THEN** within 3 seconds the app logs that the microphone stalled and opens it again
- **AND** the audio of the reopened microphone reaches the command spotting service

#### Scenario: A microphone that reports an error is opened again
- **GIVEN** listening is on
- **WHEN** the microphone reports an error
- **THEN** the app opens it again

#### Scenario: A microphone that cannot be opened again is retried
- **GIVEN** the microphone stalled
- **WHEN** opening it fails twice and then succeeds
- **THEN** the app keeps trying every second until it is open, logging the failure once

#### Scenario: The silent time keeps the capture times
- **GIVEN** the microphone was silent for 3 seconds before it was opened again
- **WHEN** the reopened microphone delivers audio
- **THEN** that audio is streamed after 3 seconds of silence, so its time in the stream is its capture time

### Requirement: Every utterance with a command heard is kept in the command history
With the command history on, the command spotting service SHALL write one command history entry for every utterance in which the keyword spotter spotted a selected phrase or the service reported a command, holding the audio the speech recognition service for commands heard, every phrase spotted in it, the transcript, the commands reported, which of them the speech recognition service heard, and the outcome: agreed, corrected, rejected or missed. An utterance with neither SHALL leave no entry.

#### Scenario: A command both models heard is kept as agreed
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the user says "press enter" and the keyword spotter spots it
- **THEN** a new command history entry holds playable audio of the utterance, the spotter's "press enter", the transcript and the reported "press enter", and its outcome is agreed

#### Scenario: A command only the speech recognition service heard is kept as missed
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the user says a command the keyword spotter does not spot and the speech recognition service hears it
- **THEN** a new command history entry names no spotted phrase, holds the transcript and the reported command, and its outcome is missed

#### Scenario: A spotted phrase the speech recognition service does not hear is kept as rejected
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the keyword spotter spots a phrase in an utterance in which the speech recognition service hears no command
- **THEN** a new command history entry holds the spotted phrase and the transcript, no command the speech recognition service heard, and its outcome is rejected

#### Scenario: A spotted phrase heard as another command is kept as corrected
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the keyword spotter spots one phrase in an utterance in which the speech recognition service hears another command
- **THEN** a new command history entry holds the spotted phrase, the transcript and the reported command, and its outcome is corrected

#### Scenario: Ordinary speech leaves no entry
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the user says a sentence with no command in it and the keyword spotter spots nothing
- **THEN** the command history holds no new entry

### Requirement: The command history is on for the app's own services and off elsewhere
The app SHALL turn the command history on by default, into a folder under the app's home folder beside the dictation archive, for the copies of the command spotting service it starts itself, and SHALL let the user turn it off in the configuration. A command spotting service not given a command history folder SHALL write no entry, so the service scripts, tests and benchmarks never write into the user's command history.

#### Scenario: The app's own services keep the command history by default
- **GIVEN** the default configuration with voice control on
- **WHEN** the app starts the copies of the command spotting service
- **THEN** each copy is told to keep the command history in a folder under the app's home folder

#### Scenario: Command history turned off
- **GIVEN** the user turned the command history off and voice control is on
- **WHEN** the app starts the copies of the command spotting service
- **THEN** no copy is told to keep a command history

#### Scenario: A service given no folder writes no entry
- **GIVEN** a command spotting service started without a command history folder
- **WHEN** a command is said to it
- **THEN** it reports the command and writes no command history entry

### Requirement: The command history stays within its size limit
After writing an entry, the command spotting service SHALL remove the oldest command history entries until the command history fits its size limit, which is the dictation archive's size limit, never removing the entry just written.

#### Scenario: The oldest command history entries go first
- **GIVEN** a command history whose entries already fill its size limit
- **WHEN** a new entry is written
- **THEN** the oldest entries are removed until the command history fits, and the newest entries, the new one included, stay

### Requirement: Keeping the command history never costs a command
The command spotting service SHALL report every command as it does with the command history off, and a failure to write or prune the command history SHALL be logged as a warning without changing what is reported or stopping the service.

#### Scenario: The command history folder cannot be written
- **GIVEN** a command history folder the service cannot write to and voice control is listening
- **WHEN** the user says "press enter"
- **THEN** Return is pressed as usual and the service log warns that the utterance was not kept

### Requirement: A doubled command needs no pause
The system SHALL carry out a doubled short form ("start start", "restart restart", "stop stop", "cancel cancel", "enter enter", "clear clear", "paste paste") as soon as its second word is heard, without waiting for a pause after it or for the next word, whether silence or speech follows.

#### Scenario: Start start then silence starts at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and stays silent
- **THEN** the start cue comes within 0.5 s of the end of "start start"

#### Scenario: Stop stop in running speech stops at once
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence, "stop stop" and goes on speaking
- **THEN** the stop sound plays within 0.5 s of the end of "stop stop", while the user is still speaking

#### Scenario: Start start inside a sentence starts at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** someone says "fine start start" and goes on with a sentence without a pause
- **THEN** the start cue comes within 0.5 s of the end of "start start", while the sentence is still being said

### Requirement: Commands can run without the Whisper confirmation
The system SHALL let the user switch off the confirmation of commands by the speech recognition service for commands; it is on by default. While it is off, a command the keyword spotter hears said on its own or doubled SHALL be carried out without waiting for that service; a command the keyword spotter cannot place on its own (inside a sentence, opening or ending one) SHALL still wait for the confirmation.

#### Scenario: Start start without confirmation starts at once
- **GIVEN** the confirmation of commands is off
- **AND** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and stays silent
- **THEN** a dictation starts
- **AND** the start cue comes within 0.3 s of the end of "start start"

#### Scenario: A command inside a sentence still waits for confirmation
- **GIVEN** the confirmation of commands is off
- **AND** a dictation runs
- **WHEN** the user says a sentence ending in "stop dictation" without a pause
- **THEN** the dictation ends only after the speech recognition service for commands heard "stop dictation"

#### Scenario: Confirmation is on by default
- **GIVEN** the user never changed the confirmation of commands
- **WHEN** the user says "start" on its own
- **THEN** the dictation starts only after the speech recognition service for commands heard "start"

### Requirement: A command carried out without confirmation records whether Whisper agreed
While the confirmation of commands is off, the speech recognition service for commands SHALL still hear every command carried out without it, after the command and without delaying it, and the log line of the command and its command history entry SHALL record whether that service heard the same command, another command or none.

#### Scenario: A command without confirmation records whether Whisper agreed
- **GIVEN** the confirmation of commands is off
- **AND** listening is on
- **WHEN** the user says "enter" on its own
- **THEN** the Return key is pressed once
- **AND** the log and the command history show that Whisper heard "enter" too

#### Scenario: A false spotting without confirmation is recorded
- **GIVEN** the confirmation of commands is off
- **AND** listening is on
- **WHEN** the keyword spotter takes a word of ordinary speech said on its own for "enter"
- **THEN** the Return key is pressed
- **AND** the log and the command history show that Whisper heard no command
