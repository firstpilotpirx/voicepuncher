## ADDED Requirements

### Requirement: In Claude, a voice start starts Claude's own dictation
When a dictation starts by voice while the Claude desktop app is the frontmost app and the Claude dictation is turned on, the system SHALL press Claude's dictation shortcut instead of recording, with the start sound and overlay of its own dictation, and SHALL NOT record, transcribe, paste or archive that dictation. In any other frontmost app a voice start SHALL start the app's own dictation as before. The choice SHALL be taken at each start and kept until that dictation ends.

#### Scenario: Saying start in Claude starts Claude's dictation
- **GIVEN** listening is on and the Claude dictation is turned on
- **AND** the Claude desktop app is the frontmost app
- **WHEN** the user says "start"
- **THEN** Claude's dictation shortcut is pressed once
- **AND** the start sound plays and the overlay shows the recording
- **AND** the app's own recorder does not start
- **AND** the log times the voice start as for the app's own dictation

#### Scenario: Another app keeps the app's own dictation
- **GIVEN** listening is on and the Claude dictation is turned on
- **AND** another app is the frontmost app
- **WHEN** the user says "start", a sentence, then "stop"
- **THEN** the app records and pastes the sentence as before
- **AND** Claude's dictation shortcut is never pressed

### Requirement: In Claude, voice commands end Claude's own dictation
During a Claude dictation, "stop" SHALL press the shortcut again, play the stop sound and hide the overlay; "cancel" SHALL press Escape, play the cancel sound and hide the overlay; "restart" SHALL press Escape, then the shortcut, with the restart sound; "press enter" SHALL press the shortcut, wait for Claude to insert its text, then press Return.

#### Scenario: Saying stop in Claude stops Claude's dictation
- **GIVEN** a dictation started by voice in Claude runs
- **WHEN** the user says "stop"
- **THEN** Claude's dictation shortcut is pressed a second time
- **AND** the stop sound plays and the overlay hides
- **AND** nothing is pasted and no dictation is archived

#### Scenario: Saying cancel in Claude drops Claude's dictation
- **GIVEN** a dictation started by voice in Claude runs
- **WHEN** the user says "cancel"
- **THEN** Escape is pressed
- **AND** the cancel sound plays and the overlay hides

#### Scenario: Saying restart in Claude starts Claude's dictation over
- **GIVEN** a dictation started by voice in Claude runs
- **WHEN** the user says "restart"
- **THEN** Escape is pressed, then Claude's dictation shortcut
- **AND** the restart sound plays and the dictation goes on

#### Scenario: Saying enter in Claude stops and sends
- **GIVEN** a dictation started by voice in Claude runs
- **WHEN** the user says "enter"
- **THEN** Claude's dictation shortcut is pressed, then Return after a pause

### Requirement: The Claude dictation falls back to the app's own dictation
The system SHALL start its own dictation, and log why, when the Claude dictation is turned off in the configuration, or when Claude's dictation shortcut cannot be posted; a configured shortcut that is not Command with one letter or digit SHALL be refused when the configuration is loaded or saved.

#### Scenario: The Claude dictation turned off records as before
- **GIVEN** the Claude dictation is turned off
- **AND** the Claude desktop app is the frontmost app
- **WHEN** a dictation starts
- **THEN** the app's own recorder starts
- **AND** Claude's dictation shortcut is not pressed

#### Scenario: A shortcut that cannot be posted falls back
- **GIVEN** the Claude desktop app is the frontmost app
- **AND** posting Claude's dictation shortcut fails
- **WHEN** a dictation starts
- **THEN** the app's own recorder starts
- **AND** the log names why the Claude dictation was not used

#### Scenario: A shortcut without Command is refused
- **WHEN** the configuration sets the Claude dictation shortcut to "d"
- **THEN** loading the configuration fails, naming the shortcut

## MODIFIED Requirements

### Requirement: "start dictation" starts a dictation
While listening and no dictation runs, the system SHALL start a dictation when the user says "start", exactly as a hotkey press starts a recording in push-to-talk, with the same start sound and overlay. Ending it by voice or by hotkey SHALL play the stop sound and show the processing overlay, and cancelling it SHALL play the cancel sound and hide the overlay, as in push-to-talk. A Claude dictation SHALL follow its own requirements.

#### Scenario: Saying start dictation starts a dictation
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start"
- **THEN** a dictation starts
- **AND** the start sound plays
- **AND** the overlay shows the recording

#### Scenario: Saying stop dictation plays the stop sound and shows processing
- **GIVEN** the user started a dictation by voice and said a sentence
- **WHEN** the user says "stop"
- **THEN** the stop sound plays
- **AND** the overlay shows processing until the sentence is pasted, then hides

### Requirement: "stop dictation" ends a dictation and pastes its text
During a dictation, the system SHALL end the dictation when the user says "stop", then transcribe and paste the text as a push-to-talk recording does. The spoken word "stop" SHALL NOT appear in the pasted text. Listening SHALL stay on afterwards. A Claude dictation SHALL follow its own requirements.

#### Scenario: Saying stop dictation pastes the dictated text
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop"
- **THEN** the sentence is pasted into the focused app
- **AND** the pasted text does not contain the stop phrase
- **AND** listening stays on

### Requirement: "press enter" during a dictation pastes and sends
During a dictation, the system SHALL take "enter", in any of its forms, as "stop dictation" followed by "press enter": it ends the dictation, pastes its text without the command, and then presses Return, with the enter sound and sign once Return is pressed. A Claude dictation SHALL follow its own requirements.

#### Scenario: Saying enter during a dictation pastes and sends
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "enter"
- **THEN** the sentence is pasted into the focused app
- **AND** the Return key is pressed after it
- **AND** the enter sound plays and the overlay shows the enter sign after the paste

### Requirement: The hotkey ends a dictation started by voice
With voice control on, a hotkey press in toggle mode, or its release in hold mode, during a dictation SHALL end the dictation as "stop dictation" does and paste its text, with the stop sound, whether the hotkey or "start dictation" started it. Listening SHALL stay on. A Claude dictation SHALL follow its own requirements.

#### Scenario: A hotkey press ends a voice dictation and keeps listening
- **GIVEN** the user started a dictation by saying "start"
- **AND** the user has said a sentence in it
- **WHEN** the user presses the hotkey
- **THEN** the stop sound plays
- **AND** the sentence is pasted into the focused app
- **AND** listening stays on

### Requirement: Push-to-talk works with voice control on or off
The system SHALL keep voice control off unless the config file or a command-line flag turns it on. Whether voice control is on or off, the hotkey SHALL record as push-to-talk does: a press starts a dictation, a press in toggle mode or the release in hold mode ends it and pastes its text, and Escape cancels it. A Claude dictation SHALL follow its own requirements.

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

### Requirement: Push-to-talk keeps its sounds and overlay
A recording started and ended by the hotkey SHALL play the sounds and show the overlay it shows today — the recording, then processing, then the Pasted sign — with no command sign; Escape during it SHALL play the cancel sound and hide the overlay. A Claude dictation SHALL follow its own requirements.

#### Scenario: A hotkey dictation shows no command sign
- **GIVEN** listening is on
- **WHEN** the user records a sentence with the hotkey
- **THEN** the start and stop sounds play
- **AND** the overlay shows the recording, then processing, then the Pasted sign, then hides, with no command sign

### Requirement: Voice commands end a dictation the hotkey started
With voice control on, the system SHALL listen for the dictation commands during a dictation the hotkey started, so "stop" ends it and pastes its text without the command, and "cancel" discards it. A Claude dictation SHALL follow its own requirements.

#### Scenario: Saying stop dictation ends a hotkey dictation
- **GIVEN** voice control is turned on
- **AND** the user started a dictation with the hotkey and said a sentence
- **WHEN** the user says "stop"
- **THEN** the sentence is pasted into the focused app without "stop"

#### Scenario: Saying cancel dictation discards a hotkey dictation
- **GIVEN** voice control is turned on
- **AND** the user started a dictation with the hotkey and said a sentence
- **WHEN** the user says "cancel"
- **THEN** nothing is pasted
- **AND** the cancel sound plays
- **AND** listening stays on

### Requirement: A dictation started by voice holds the speech right after the command
When a voice command starts a dictation ("start" said on its own, opening an utterance, doubled, or last in a chain, or "restart"), the system SHALL begin the dictation where the command's words end, not when the command was recognized. So all the speech said after the command, its first words included, SHALL be part of the dictation. The command words SHALL NOT be pasted. A Claude dictation SHALL follow its own requirements.

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
- **WHEN** the user says "restart restart", pauses for about 0.5 s and says another sentence, and later ends the dictation
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

### Requirement: "start" opening speech starts the dictation with it
The system SHALL start a dictation when "start" opens an utterance and speech follows it without a pause, while no dictation runs; the speech after the command SHALL be part of the dictation and the command words SHALL NOT be pasted. Another command that opens an utterance followed by speech SHALL take effect at once like any command word; "start" followed at once by "stop" starts and at once ends a dictation that pastes nothing. A Claude dictation SHALL follow its own requirements.

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

### Requirement: A whole hands-free session runs as said
The system SHALL carry out a long hands-free session — dictations ended by "stop" or "cancel", Return presses and clears, in any order — exactly as said: each stopped dictation pasted with its own words and without the command phrase, each cancelled one dropped, each key press done once, in order. A Claude dictation SHALL follow its own requirements.

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
