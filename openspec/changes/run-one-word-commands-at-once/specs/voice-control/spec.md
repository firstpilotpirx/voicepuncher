## ADDED Requirements

### Requirement: A command counts wherever its word is said
The system SHALL carry out a command as soon as its word is heard, wherever it is said in an utterance — on its own, at the end of other speech, at its start, or inside a sentence with speech after it — when the state accepts the command; no pause before or after it is needed. The user keeps the command words out of dictation, so a command word inside a sentence counts as the command. Ordinary speech without a command word triggers nothing.

#### Scenario: A command phrase spoken on its own is recognized
- **GIVEN** listening is on
- **WHEN** the user says a command phrase with a pause before and after it
- **THEN** that command takes effect

#### Scenario: A command said after a long time of listening is recognized
- **GIVEN** listening has been on for a long time with nothing said
- **WHEN** the user says a command phrase with a pause before and after it
- **THEN** that command takes effect

#### Scenario: A command word inside a sentence counts while listening
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** someone says "I will enter the code when I am done" without pausing
- **THEN** the Return key is pressed in the focused app

#### Scenario: Ordinary speech triggers no command
- **GIVEN** listening is on
- **WHEN** someone speaks ordinary Russian and English sentences that hold no command phrase
- **THEN** no command takes effect

#### Scenario: A stop word inside a dictated sentence ends the dictation at once
- **GIVEN** a dictation runs
- **WHEN** the user says "please stop the software from crashing" without pausing
- **THEN** the stop sound plays before the sentence ends
- **AND** "please" is pasted into the focused app, without "stop" and the words after it

### Requirement: A command ending speech counts without a pause
The system SHALL carry out a command whose word ends an utterance, even with speech right before it in the same utterance, when the state accepts the command, as soon as the word is heard: no pause after it is awaited. When the command ends a dictation, the pasted text SHALL hold the speech before the command and SHALL NOT hold the command words.

#### Scenario: Stop right after the last dictated word
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence and then "stop" without a pause, then pauses
- **THEN** the sentence is pasted into the focused app without "stop"

#### Scenario: Enter right after other speech
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "okay, enter" without a pause, then pauses
- **THEN** the Return key is pressed in the focused app

#### Scenario: Stop after a long dictation counts without waiting for a pause
- **GIVEN** a dictation runs
- **AND** the user has spoken in it for more than 10 s without a pause
- **WHEN** the user says "stop" right after the last word
- **THEN** the stop sound plays within 0.6 s of the end of "stop"

### Requirement: "start" opening speech starts the dictation with it
The system SHALL start a dictation when "start" opens an utterance and speech follows it without a pause, while no dictation runs; the speech after the command SHALL be part of the dictation and the command words SHALL NOT be pasted. Another command that opens an utterance followed by speech SHALL take effect at once like any command word; "start" followed at once by another command word ("start stop") SHALL have no effect.

#### Scenario: Start followed by the dictation at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start" and a sentence without a pause, and later ends the dictation
- **THEN** the sentence is pasted into the focused app without "start"

#### Scenario: Enter followed by speech presses Return
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter the building on the left" and pauses
- **THEN** the Return key is pressed in the focused app

### Requirement: Every command is said by its one word
The system SHALL take each command only from its one word or that word said twice in a row: "start" for "start dictation", "stop" for "stop dictation", "cancel" for "cancel dictation", "restart" for "restart dictation", "enter" for "press enter", "clear" for "clear all text" and "paste" for "paste text", in the state that accepts the command. The longer phrases "start dictation", "stop dictation", "cancel dictation", "restart dictation", "press enter", "clear all text", "clear text" and "paste text" SHALL never be a command of their own: each counts only by its command word, like any speech that holds it; the command names stay only as names.

#### Scenario: A short form is recognized as its command
- **GIVEN** listening is on
- **WHEN** the user says "start", "stop", "cancel", "restart", "enter", "clear" or "paste" on its own, in the state that accepts its command
- **THEN** "start dictation", "stop dictation", "cancel dictation", "restart dictation", "press enter", "clear all text" or "paste text" takes effect

#### Scenario: Cancel alone discards a dictation
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "cancel"
- **THEN** no text is pasted

#### Scenario: A long form counts by its command word
- **GIVEN** a dictation runs
- **WHEN** the user says "stop dictation" on its own with a pause before and after it
- **THEN** the dictation ends, as for any speech holding "stop"
- **AND** "dictation" is not pasted

#### Scenario: A long form ending in a command word counts by that word
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "press enter" on its own
- **THEN** the Return key is pressed, as for any utterance ending in "enter"

### Requirement: "paste" pastes the clipboard at once
While listening, with or without a dictation running, the system SHALL press Cmd+V in the focused app when the user says "paste", so the focused app pastes whatever the clipboard holds at its cursor: what the user copied, before the dictation or during it, never the dictated text. During a dictation the command SHALL NOT end, pause or restart the dictation: it keeps recording, and on its end pastes its own text, without the words of the command, where the cursor then is. The command SHALL leave the clipboard as it is. It SHALL then play the paste sound and show the paste sign on the overlay. With an empty clipboard the focused app pastes nothing, and the command still confirms itself.

#### Scenario: Saying paste text pastes the clipboard at the cursor
- **GIVEN** listening is on
- **AND** no dictation runs
- **AND** the clipboard holds a sentence
- **WHEN** the user says "paste"
- **THEN** Cmd+V is pressed in the focused app
- **AND** the paste sound plays
- **AND** the overlay shows the paste sign for about a second, then hides

#### Scenario: Paste leaves the clipboard as it was
- **GIVEN** listening is on
- **AND** no dictation runs
- **AND** the clipboard holds a sentence
- **WHEN** the user says "paste"
- **THEN** the clipboard still holds the same sentence

#### Scenario: A paste word inside a sentence pastes at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** someone says "I will paste the link later" without pausing
- **THEN** Cmd+V is pressed in the focused app

#### Scenario: Saying paste during a dictation pastes the clipboard and the dictation goes on
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **AND** the clipboard holds another sentence the user copied
- **WHEN** the user says "paste" with a pause before and after it
- **THEN** Cmd+V is pressed in the focused app
- **AND** the paste sound plays
- **AND** the dictation keeps running

#### Scenario: A dictation with a paste in it pastes its text without the command
- **GIVEN** a dictation runs
- **AND** the user has said a sentence, then "paste" on its own, then another sentence
- **WHEN** the user says "stop"
- **THEN** both dictated sentences are pasted into the focused app
- **AND** the pasted text does not contain "paste"

### Requirement: Streaming commands are reported as soon as their words are decoded
With the streaming recognizer on an idle machine, the command spotting service SHALL report "start" and "stop" within 0.3 s of the end of their words (wall clock) for at least half of them and within 0.6 s for 9 of 10. A command word SHALL count as soon as a recognizer has decoded it, wherever it sits, without waiting for the gate's pause or for the words after it, unless it may still be the start of a longer command word the state listens for ("start" before "restart" is decoded whole), which counts once decoded past it. A one-word command only the English recognizer hears, where the Russian recognizer heard another word, SHALL count only at the gate's pause right after it, and not at the head of a sentence that goes on: a Russian word the English recognizer hears as "stop" is no command.

#### Scenario: Start said alone is reported within its bound
- **GIVEN** the command spotting service runs with the streaming recognizer on an otherwise idle machine
- **WHEN** an app streams "start" followed by silence, in real time
- **THEN** the service reports "start dictation" within 0.3 s of the end of "start" by the wall clock

#### Scenario: Stop said alone is reported at once
- **GIVEN** the command spotting service runs with the streaming recognizer on an otherwise idle machine
- **WHEN** an app streams "stop" followed by silence, in real time
- **THEN** the service reports "stop dictation" within 0.3 s of the end of "stop" by the wall clock

#### Scenario: Stop opening a sentence counts at once
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "stop the car" without a pause
- **THEN** the service reports "stop dictation" before the end of "car"

#### Scenario: A Russian sentence heard as opening with stop is no command
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** the user's own recordings exist on the machine, among them "чтобы зайти в гетлаб" said while dictating, whose first word the English recognizer hears as "stop"
- **WHEN** an app streams that recording in real time, listening for the commands of a dictation
- **THEN** the service reports no command

#### Scenario: Press enter after a sentence is reported without waiting for the pause
- **GIVEN** the command spotting service runs with the streaming recognizer on an otherwise idle machine
- **WHEN** an app streams a sentence ending with "enter", followed by silence, in real time
- **THEN** the service reports "press enter" within 0.6 s of the end of "enter" by the wall clock

#### Scenario: The user's recorded commands are reported within their bound
- **GIVEN** the user's own recordings of the commands exist on the machine
- **WHEN** each recorded "start" and "stop" is streamed to the service in real time
- **THEN** at least half are reported within 0.3 s and at least 9 of 10 within 0.6 s of the end of their words

#### Scenario: A command after speech in the user's recordings is reported soon after its word
- **GIVEN** the user's own recordings exist on the machine, among them commands said right after speech
- **WHEN** each is streamed to the service in real time
- **THEN** the median delay of those commands after the end of their words is at most 0.4 s
## MODIFIED Requirements

### Requirement: A doubled command needs no pause
The system SHALL carry out a doubled short form ("start start", "restart restart", "stop stop", "cancel cancel", "enter enter", "clear clear", "paste paste") as one command, at once, whether silence or speech follows: its first word already counts, and the second, said within 0.6 s of the first, is the same command — never carried out again and never part of a dictation's text. A doubled form said inside a sentence SHALL take effect within 1 s of its end, before the sentence ends. A dictation a doubled start or restart begins runs from the end of the first word, and the second word is removed from its text.

#### Scenario: Start start then silence starts at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and stays silent
- **THEN** the start cue comes within 0.5 s of the end of "start start"

#### Scenario: Stop stop in running speech stops at once
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence, "stop stop" and goes on speaking
- **THEN** the stop sound plays within 1 s of the end of "stop stop", while the user is still speaking

#### Scenario: Start start inside a sentence starts at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** someone says "fine start start" and goes on with a sentence without a pause
- **THEN** the start cue comes within 1 s of the end of "start start", while the sentence is still being said

#### Scenario: Enter enter presses Return once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter enter" and pauses
- **THEN** the Return key is pressed once

## REMOVED Requirements

### Requirement: A command counts on its own or in its set places
**Reason**: a command now counts as soon as its word is heard, wherever it is said; replaced by "A command counts wherever its word is said".
**Migration**: none; the user keeps command words out of dictation.

### Requirement: A command may end an utterance
**Reason**: a command now counts as soon as its word is heard, wherever it is said; replaced by "A command ending speech counts without a pause".
**Migration**: none; the user keeps command words out of dictation.

### Requirement: "start" may open an utterance
**Reason**: a command now counts as soon as its word is heard, wherever it is said; replaced by ""start" opening speech starts the dictation with it".
**Migration**: none; the user keeps command words out of dictation.

### Requirement: Every command has a one-word short form
**Reason**: a command now counts as soon as its word is heard, wherever it is said; replaced by "Every command is said by its one word".
**Migration**: none; the user keeps command words out of dictation.

### Requirement: "paste text" pastes the clipboard
**Reason**: a command now counts as soon as its word is heard, wherever it is said; replaced by ""paste" pastes the clipboard at once".
**Migration**: none; the user keeps command words out of dictation.

### Requirement: Streaming commands are reported within 0.3 s of their words
**Reason**: a command now counts as soon as its word is heard, wherever it is said; replaced by "Streaming commands are reported as soon as their words are decoded".
**Migration**: none; the user keeps command words out of dictation.
