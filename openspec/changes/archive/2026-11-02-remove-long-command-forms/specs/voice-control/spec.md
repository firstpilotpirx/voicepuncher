## MODIFIED Requirements

### Requirement: "start dictation" starts a dictation
While listening and no dictation runs, the system SHALL start a dictation when the user says "start", exactly as a hotkey press starts a recording in push-to-talk, with the same start sound and overlay. Ending it by voice or by hotkey SHALL play the stop sound and show the processing overlay, and cancelling it SHALL play the cancel sound and hide the overlay, as in push-to-talk.

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
During a dictation, the system SHALL end the dictation when the user says "stop", then transcribe and paste the text as a push-to-talk recording does. The spoken word "stop" SHALL NOT appear in the pasted text. Listening SHALL stay on afterwards.

#### Scenario: Saying stop dictation pastes the dictated text
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop"
- **THEN** the sentence is pasted into the focused app
- **AND** the pasted text does not contain the stop phrase
- **AND** listening stays on

### Requirement: "cancel dictation" discards a dictation
During a dictation, the system SHALL discard the dictation when the user says "cancel": nothing is pasted and the cancel sound plays, as when a push-to-talk recording is cancelled. Listening SHALL stay on afterwards.

#### Scenario: Saying cancel dictation pastes nothing
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "cancel"
- **THEN** nothing is pasted
- **AND** the cancel sound plays
- **AND** listening stays on

### Requirement: "press enter" presses Return
While listening and no dictation runs, the system SHALL press the Return key in the focused app when the user says "enter". It SHALL then play the enter sound and show the enter sign on the overlay.

#### Scenario: Saying press enter presses Return
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter"
- **THEN** the Return key is pressed in the focused app
- **AND** the enter sound plays
- **AND** the overlay shows the enter sign for about a second, then hides

### Requirement: "clear all text" erases the focused text
While listening and no dictation runs, the system SHALL select all and delete in the focused app when the user says "clear": it presses Cmd+A, then Delete. It SHALL then play the clear sound and show the clear sign on the overlay.

#### Scenario: Saying clear all text selects all and deletes
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "clear"
- **THEN** Cmd+A and then Delete are pressed in the focused app
- **AND** the clear sound plays
- **AND** the overlay shows the clear sign for about a second, then hides

### Requirement: Commands are carried out in the order spoken
The system SHALL carry out commands in the order they were spoken. A command spoken while the previous dictation is still being transcribed or pasted SHALL wait until that text is pasted.

#### Scenario: Press enter right after stop dictation follows the paste
- **GIVEN** a dictation runs
- **AND** the user has said "stop"
- **WHEN** the user says "enter" before the text is pasted
- **THEN** the text is pasted first
- **AND** the Return key is pressed after it

#### Scenario: A message is dictated and sent by voice alone
- **GIVEN** listening is on
- **WHEN** the user says "start", a sentence, "stop" and "enter", each command on its own
- **THEN** the sentence is pasted into the focused app
- **AND** the Return key is pressed after it

### Requirement: A command said twice in one utterance counts once
The system SHALL carry out a command once when the user repeats it within one utterance, with a pause shorter than the 0.3 s that ends an utterance ("enter, enter"). A longer pause separates two commands; a repeated "start", "stop" or "cancel" then has no effect, since the state it leads to does not accept it.

#### Scenario: A repeated press enter presses Return once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter, enter" without pausing
- **THEN** the Return key is pressed once

### Requirement: The user's own way of saying the commands is recognized
The system SHALL recognize the commands as the user says them, including small slips of pronunciation or of the speech recognition service ("and restart" for a quick "enter start" closing a chain). A command said on its own SHALL be recognized whatever the length of the pause before it, and in a quiet voice, as quiet as the user's quietest recorded commands, as well as at a normal level. Every command the user said in a recording kept for testing SHALL be recognized exactly once, apart from recordings marked unclear; repeats said without a pause of 0.3 s are one command. A short Russian phrase SHALL NOT be taken for a command; "стоп" is left out of this rule, since the speech recognition service may hear it as the short form "stop".

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
- **WHEN** the user says a command word on its own in a quiet voice, in the state that accepts the command
- **THEN** that command takes effect

#### Scenario: A quiet clear all text clears the text
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "clear" on its own, as quiet as the user's quietest recorded commands, with a pause before and after it
- **THEN** the focused text is selected and deleted once

#### Scenario: Cancel dictation heard as capsule dictation discards the dictation
- **GIVEN** a dictation runs
- **WHEN** the user says "capsule dictation" on its own, or the speech recognition service hears it in a running dictation
- **THEN** it is ordinary speech: the dictation is not discarded and goes on recording

### Requirement: A whole hands-free session runs as said
The system SHALL carry out a long hands-free session — dictations ended by "stop" or "cancel", Return presses and clears, in any order — exactly as said: each stopped dictation pasted with its own words and without the command phrase, each cancelled one dropped, each key press done once, in order.

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

### Requirement: The hotkey ends a dictation started by voice
With voice control on, a hotkey press in toggle mode, or its release in hold mode, during a dictation SHALL end the dictation as "stop dictation" does and paste its text, with the stop sound, whether the hotkey or "start dictation" started it. Listening SHALL stay on.

#### Scenario: A hotkey press ends a voice dictation and keeps listening
- **GIVEN** the user started a dictation by saying "start"
- **AND** the user has said a sentence in it
- **WHEN** the user presses the hotkey
- **THEN** the stop sound plays
- **AND** the sentence is pasted into the focused app
- **AND** listening stays on

### Requirement: Voice commands end a dictation the hotkey started
With voice control on, the system SHALL listen for the dictation commands during a dictation the hotkey started, so "stop" ends it and pastes its text without the command, and "cancel" discards it.

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
- **AND** saying "enter" presses Return

### Requirement: A command counts on its own or in its set places
The system SHALL carry out a command when its phrase is the whole utterance, with a pause before and after it, and in the places the requirements on commands said without a pause allow: at the end of an utterance, "start" at its start, a doubled short form or a chain anywhere. The same words inside a sentence — with speech after them in the same utterance, not doubled and not in a chain — SHALL NOT be taken as a command; a single short word right after a command ("enter now") is within the spotter's timing error and may still count.

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
- **WHEN** someone says "I will enter the code when I am done" without pausing
- **THEN** no key is pressed
- **AND** no dictation starts

#### Scenario: Ordinary speech triggers no command
- **GIVEN** listening is on
- **WHEN** someone speaks ordinary Russian and English sentences that hold no command phrase
- **THEN** no command takes effect

#### Scenario: A stop phrase inside a dictated sentence does not end the dictation
- **GIVEN** a dictation runs
- **WHEN** the user says "please stop the software from crashing" without pausing
- **THEN** the dictation keeps running

### Requirement: "press enter" during a dictation pastes and sends
During a dictation, the system SHALL take "enter", in any of its forms, as "stop dictation" followed by "press enter": it ends the dictation, pastes its text without the command, and then presses Return, with the enter sound and sign once Return is pressed.

#### Scenario: Saying enter during a dictation pastes and sends
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "enter"
- **THEN** the sentence is pasted into the focused app
- **AND** the Return key is pressed after it
- **AND** the enter sound plays and the overlay shows the enter sign after the paste

### Requirement: Every command has a one-word short form
The system SHALL take each command only from its one word or that word said twice in a row: "start" for "start dictation", "stop" for "stop dictation", "cancel" for "cancel dictation", "restart" for "restart dictation", "enter" for "press enter", "clear" for "clear all text" and "paste" for "paste text", in the state that accepts the command. The longer phrases "start dictation", "stop dictation", "cancel dictation", "restart dictation", "press enter", "clear all text", "clear text" and "paste text" SHALL be ordinary speech, never a command of their own; the command names stay only as names.

#### Scenario: A short form is recognized as its command
- **GIVEN** listening is on
- **WHEN** the user says "start", "stop", "cancel", "restart", "enter", "clear" or "paste" on its own, in the state that accepts its command
- **THEN** "start dictation", "stop dictation", "cancel dictation", "restart dictation", "press enter", "clear all text" or "paste text" takes effect

#### Scenario: Cancel alone discards a dictation
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "cancel"
- **THEN** no text is pasted

#### Scenario: A long form said alone takes no effect
- **GIVEN** listening is on
- **WHEN** the user says "stop dictation" during a dictation, on its own with a pause before and after it
- **THEN** no command takes effect

#### Scenario: A long form ending in a command word counts by that word
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "press enter" on its own
- **THEN** the Return key is pressed, as for any utterance ending in "enter"

### Requirement: "restart dictation" starts the dictation over
During a dictation, the system SHALL discard what was dictated so far, paste nothing, and start a new dictation at once when the user says "restart". It plays the restart sound alone, neither the cancel sound nor the start sound, and the overlay shows the restart sign for about a second, then the recording again. "restart restart" counts anywhere, like the other doubled short forms, and "restart" may end an utterance. It is in no chain: it only follows a running dictation, and nothing follows it in one breath.

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
The system SHALL carry out a command whose phrase (full or short) ends an utterance, even with speech right before it in the same utterance, when the state accepts the command. The utterance ends with a pause of at least 0.3 s after the command. When the command ends a dictation, the pasted text SHALL hold the speech before the command and SHALL NOT hold the command words.

#### Scenario: Stop right after the last dictated word
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence and then "stop" without a pause, then pauses
- **THEN** the sentence is pasted into the focused app without "stop"

#### Scenario: Enter right after other speech
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "okay, enter" without a pause, then pauses
- **THEN** the Return key is pressed in the focused app

#### Scenario: A command word inside a sentence is still no command
- **GIVEN** a dictation runs
- **WHEN** the user says "we should stop the server tonight" and pauses
- **THEN** the dictation keeps running

### Requirement: "start" may open an utterance
The system SHALL start a dictation when "start" opens an utterance and speech follows it without a pause, while no dictation runs; the speech after the command SHALL be part of the dictation and the command words SHALL NOT be pasted. Other commands that open an utterance followed by speech SHALL have no effect, and so SHALL "start" followed at once by another command word ("start stop").

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

### Requirement: An utterance ends after 0.3 s of silence
The system SHALL end an utterance after 0.3 s of silence, so a command said after a short breath is an utterance of its own, also in a noisy room where the silence is the room's own noise. This SHALL hold for a quiet voice and when speech follows the pause at once: a command said on its own, followed by 0.3–0.6 s of silence and then speech, SHALL take effect. The speech after the pause SHALL NOT change what the command is heard as. A pause is the silence between the words as said, not the silence a speech synthesizer adds after a word.

#### Scenario: Stop after a short breath
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence, pauses for about 0.35 s and says "stop stop", then pauses
- **THEN** the sentence is pasted into the focused app without "stop"

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
When a voice command starts a dictation ("start" said on its own, opening an utterance, doubled, or last in a chain, or "restart"), the system SHALL begin the dictation where the command's words end, not when the command was recognized. So all the speech said after the command, its first words included, SHALL be part of the dictation. The command words SHALL NOT be pasted.

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

### Requirement: Listening comes back when the command spotting service is back
When the command spotting service answers again after it went away, the system SHALL resume listening for the commands of the current state within 5 seconds and log that listening is back, without a restart of the app and without playing a sound. The system SHALL NOT give up reaching for the service however many attempts fail: an outage that outlasts the speech it replays and the tenth failed attempt still ends with listening back.

#### Scenario: Commands work again after the command spotting service restarts
- **GIVEN** listening is on and no dictation runs
- **AND** the command spotting service went away and was started again
- **WHEN** the user says "enter" 5 seconds after the service answers again
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
- **WHEN** the user keeps dictating through that hold and then says "stop"
- **THEN** listening is still on and no error about the command spotting service is logged
- **AND** the dictation ends and its text is pasted

#### Scenario: A pause of the app's own does not lose the service
- **GIVEN** listening is on and the command spotting service answers every check at once
- **WHEN** the app reads nothing from the service for 6 seconds, as when it is busy or the computer sleeps
- **THEN** the service is not reported lost and no error about the command spotting service is logged

#### Scenario: A burst of audio after long listening does not lose the service
- **GIVEN** listening is on with a freshly started command spotting service and the machine is busy
- **WHEN** 30 seconds of silence followed by "start" reach the app at once
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
- **WHEN** the user says "stop"
- **THEN** the dictation ends and its text is pasted

### Requirement: A command said while the command spotting service was away still counts
The system SHALL send the service the speech heard during the last 10 seconds of an outage once the service is back, so a command said in that time is carried out late rather than lost. Microphone audio that never reached the app before the outage SHALL NOT shift the speech sent again: it still begins before the moment the service went away.

#### Scenario: "stop dictation" said during a short outage ends the dictation
- **GIVEN** a dictation runs
- **AND** the command spotting service is away for 3 seconds
- **WHEN** the user says "stop" during those 3 seconds
- **THEN** the dictation ends once the service is back, and the word "stop" is not in the pasted text

#### Scenario: Speech sent again after an outage begins before the loss when microphone audio was lost earlier
- **GIVEN** listening is on and some seconds of microphone audio never reached the app
- **AND** the command spotting service went away afterwards
- **WHEN** the service is back
- **THEN** the speech sent to it again begins before the moment the service went away

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
- **WHEN** the user says "clear"
- **THEN** the picked sound plays

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
- **AND** a dictation is started by the hotkey right after the user said "start", before its confirmation comes back
- **WHEN** the confirmation of "start dictation" comes back
- **THEN** it is taken as "start dictation"
- **AND** the dictation is not restarted

#### Scenario: A start said right after a slowly confirmed stop stays a start
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 2.5 seconds to answer each confirmation
- **WHEN** the user says "stop stop" and then, before its confirmation comes back, "start"
- **THEN** the dictation ends and a new dictation starts

#### Scenario: A chain is carried out the same on a busy machine
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 2 seconds to answer each confirmation
- **AND** the machine is busy with other work, so the speech is taken in late and in bursts
- **WHEN** the user says "stop enter start"
- **THEN** the dictation ends, its text is pasted, Return is pressed and a new dictation starts, each exactly once, as on an idle machine

### Requirement: "paste text" pastes the clipboard
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

#### Scenario: A paste phrase inside a sentence is ignored
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** someone says "I will paste the link later" without pausing
- **THEN** no key is pressed

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

### Requirement: A confirmation never waits past its deadline for a copy
The command spotting service SHALL send a confirmation to a free copy of the speech recognition service for commands, or wait for one only until the confirmation's deadline; a confirmation that finds no free copy by then SHALL be given up and logged without ever being sent, so given-up confirmations never occupy a copy.

#### Scenario: Every copy for commands is busy past the deadline
- **GIVEN** voice control runs, and every copy of the speech recognition service for commands is busy for longer than a confirmation's deadline
- **WHEN** the user says "enter"
- **THEN** that confirmation is logged as given up and ends without waiting much past its deadline, and no copy receives it

#### Scenario: A copy frees before the deadline
- **GIVEN** voice control runs, and every copy of the speech recognition service for commands is busy for one more second
- **WHEN** the user says "enter"
- **THEN** the confirmation is sent to the first copy that frees, and "press enter" is reported

### Requirement: Listening moves to another copy of the command spotting service
When the copy of the command spotting service the app streams to goes away while another copy answers, the system SHALL connect to that other copy at its first attempt to reach the service again and replay the speech the lost copy may have missed, so a command said meanwhile still counts and the overlay never shows that voice commands do not work.

#### Scenario: The command spotting copy the app listens with is killed
- **GIVEN** the app started the command spotting service as two copies and listening is on
- **WHEN** the copy the app streams to is killed and the user says "enter" a moment later
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
- **WHEN** one copy is killed and the user says "enter" right after
- **THEN** the Return key is pressed in the focused app

### Requirement: A confirmation whose copy stalls goes to another copy
The command spotting service SHALL send a confirmation whose copy of the speech recognition service for commands does not answer within its decoding budget to another copy while the confirmation's deadline lasts, and SHALL never take a copy's silence for speech without a command.

#### Scenario: One copy for commands stalls mid-command
- **GIVEN** the speech recognition service for commands runs as two copies and one stops answering
- **WHEN** a confirmation of "enter" said alone reaches the stalled copy
- **THEN** the other copy confirms it within the confirmation's deadline and "press enter" is reported

#### Scenario: Every copy for commands stalls
- **GIVEN** every copy of the speech recognition service for commands stops answering
- **WHEN** a confirmation of "enter" said alone is asked
- **THEN** it ends as given up, not as speech heard without a command

### Requirement: A command whose confirmation fails is reported
The command spotting service SHALL report a closed speech segment whose confirmation got no answer from the speech recognition service for commands as a failed confirmation, and the system SHALL then show that voice commands are unavailable, within 10 seconds of the segment's end.

#### Scenario: The service reports a segment it could not confirm
- **GIVEN** every copy of the speech recognition service for commands stops answering
- **WHEN** an app streams "enter" said alone
- **THEN** the service reports that the confirmation of that segment failed, within 10 seconds of its end

#### Scenario: The app shows a failed confirmation
- **GIVEN** listening is on
- **WHEN** the command spotting service reports a failed confirmation
- **THEN** the overlay shows the failure notice that voice commands are unavailable

### Requirement: A command confirmed slowly under load is still carried out
When the speech recognition service for commands answers its confirmations slowly because the machine is busy, the command spotting service SHALL still report a command said alone or in its place, in the order spoken, within the confirmation's deadline, instead of dropping it as speech without a command.

#### Scenario: A stop right after dictated speech under load
- **GIVEN** voice control runs, a dictation is on, and the speech recognition service for commands takes over a second for every confirmation, as under machine load
- **WHEN** the user dictates a sentence and says "stop" right after its last word
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
- **WHEN** the user says "clear" alone and the app asks for a sync right after the speech
- **THEN** the command spotting service reports "clear all text" before it answers the sync

#### Scenario: A command whose half-pause hearing is misheard is reported before the sync is answered
- **GIVEN** voice control runs in standby and the speech recognition service for commands mishears a command word in the hearing asked at half the pause and hears it right in the hearing of the whole segment
- **WHEN** the user says "okay, clear" with a short pause after "okay" and the app asks for a sync right after the speech
- **THEN** the command spotting service reports "clear all text" before it answers the sync

### Requirement: Every utterance with a command heard is kept in the command history
With the command history on, the command spotting service SHALL write one command history entry for every utterance in which the keyword spotter spotted a selected phrase or the service reported a command, holding the audio the speech recognition service for commands heard, every phrase spotted in it, the transcript, the commands reported, which of them the speech recognition service heard, and the outcome: agreed, corrected, rejected or missed. An utterance with neither SHALL leave no entry.

#### Scenario: A command both models heard is kept as agreed
- **GIVEN** the command history is on and voice control is listening
- **WHEN** the user says "enter" and the keyword spotter spots it
- **THEN** a new command history entry holds playable audio of the utterance, the spotter's "enter", the transcript and the reported "press enter", and its outcome is agreed

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

### Requirement: Keeping the command history never costs a command
The command spotting service SHALL report every command as it does with the command history off, and a failure to write or prune the command history SHALL be logged as a warning without changing what is reported or stopping the service.

#### Scenario: The command history folder cannot be written
- **GIVEN** a command history folder the service cannot write to and voice control is listening
- **WHEN** the user says "enter"
- **THEN** Return is pressed as usual and the service log warns that the utterance was not kept

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
- **WHEN** the user says a sentence ending in "stop" without a pause
- **THEN** the dictation ends only after the speech recognition service for commands heard "stop"

#### Scenario: Confirmation is on by default
- **GIVEN** the user never changed the confirmation of commands
- **WHEN** the user says "start" on its own
- **THEN** the dictation starts only after the speech recognition service for commands heard "start"

### Requirement: Each state accepts its own commands, paste in both
The system SHALL accept "start dictation" and "clear all text" only with no dictation running, "stop dictation", "cancel dictation" and "restart dictation" only during a dictation, and "press enter" and "paste text" in both. A command spoken in a state that does not accept it SHALL have no effect. "start" said during a dictation, however and wherever said, SHALL neither be taken for "restart" nor end the dictation.

#### Scenario: Paste during a dictation is carried out
- **GIVEN** a dictation runs
- **WHEN** the user says "paste"
- **THEN** Cmd+V is pressed in the focused app
- **AND** the dictation keeps running

#### Scenario: Stop dictation without a dictation is ignored
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "stop"
- **THEN** nothing is pasted and no sound plays

#### Scenario: Clear during a dictation is ignored
- **GIVEN** a dictation runs
- **WHEN** the user says "clear"
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

### Requirement: Streaming recognizers hear commands behind a voice gate
With the streaming recognizer, the command spotting service SHALL run a voice gate over the audio and, from the onset of speech, feed an English and a Russian streaming recognizer the speech together with the half second before the onset, until the speech has ended. A command either recognizer hears SHALL be reported as soon as it is heard, without a Whisper confirmation, and without waiting for the other recognizer to decode the same audio. Without speech, no recognizer SHALL run. The gate SHALL open on a command said as quietly as the quietest commands in the user's own recordings.

#### Scenario: A command said alone is reported before its pause ends
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "enter" followed by half a second of silence and asks for a sync
- **THEN** the service reports "press enter" before it answers the sync

#### Scenario: Silence wakes no recognizer
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams three seconds of silence
- **THEN** the service reports no command
- **AND** its log shows that no recognizer ran

#### Scenario: Speech without a command is no command
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams the sentence "the weather is nice today"
- **THEN** the service reports no command

#### Scenario: A slow recognizer does not hold back the other
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** the English recognizer decodes slower than the audio arrives
- **WHEN** an app streams "стоп" followed by silence
- **THEN** the service reports "stop dictation" as soon as the Russian recognizer heard it, before the English recognizer has decoded the same audio

#### Scenario: A quiet command opens the gate
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "start" at the loudness of the user's quietest recorded command, followed by silence
- **THEN** the service reports "start dictation"

### Requirement: A command said in Russian sounds counts as the command
With the streaming recognizer, a command word the Russian recognizer hears in Cyrillic ("стоп", "энтер", "клиар", "старт") SHALL count as the English command it sounds like.

#### Scenario: A Russian-accented stop is heard as stop
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "стоп" said by a Russian voice followed by silence
- **THEN** the service reports "stop dictation"

#### Scenario: A Russian word that is no command stays speech
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "привет" said by a Russian voice followed by silence
- **THEN** the service reports no command

### Requirement: The first recognizer to hear a command reports it once
With the streaming recognizer, a command SHALL be reported by whichever recognizer hears it first; the same command heard by the other recognizer in the same utterance SHALL NOT be reported again, and a doubled command ("stop stop") SHALL count once.

#### Scenario: A command both recognizers hear is reported once
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "enter" followed by silence
- **THEN** the service reports "press enter" exactly once

#### Scenario: A doubled command is reported once
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "enter enter" followed by silence
- **THEN** the service reports "press enter" exactly once

### Requirement: Each streaming utterance is kept with its texts and stage times
With the streaming recognizer and a command history, the service SHALL keep every utterance the voice gate opened: its audio from the replayed half second on, each recognizer's texts with their stream times, the command and the recognizer that heard it when one was reported, the times of onset, first text, report and end of speech, and each recognizer's decode time; and SHALL log one line per utterance with those times. An utterance in which no command was reported SHALL be kept as such, so a missed command can be found in the history.

#### Scenario: A reported command leaves an entry with its timeline
- **GIVEN** the command spotting service runs with the streaming recognizer and keeps a command history
- **WHEN** an app streams "enter" followed by silence
- **THEN** the command history holds an entry with the utterance's audio
- **AND** the entry names "press enter", the recognizer that heard it, each recognizer's texts with their times and the times of each stage

#### Scenario: An utterance without a command is kept too
- **GIVEN** the command spotting service runs with the streaming recognizer and keeps a command history
- **WHEN** an app streams the sentence "the weather is nice today" followed by silence
- **THEN** the command history holds an entry with the utterance's audio and each recognizer's texts
- **AND** the entry names no command

### Requirement: Streaming commands are reported within 0.3 s of their words
With the streaming recognizer on an idle machine, the command spotting service SHALL report "start" and "stop" within 0.3 s of the end of their words (wall clock) for at least half of them and within 0.6 s for 9 of 10; a command said alone or ending a sentence SHALL count at the pause the gate hears after it, without waiting for the utterance to close, unless its last word is also the end of a longer command word the state listens for ("start" in "restart"), which counts at the close.

#### Scenario: Start said alone is reported within its bound
- **GIVEN** the command spotting service runs with the streaming recognizer on an otherwise idle machine
- **WHEN** an app streams "start" followed by silence, in real time
- **THEN** the service reports "start dictation" within 0.3 s of the end of "start" by the wall clock

#### Scenario: Stop said alone is reported at the pause after it
- **GIVEN** the command spotting service runs with the streaming recognizer on an otherwise idle machine
- **WHEN** an app streams "stop" followed by silence, in real time
- **THEN** the service reports "stop dictation" within 0.3 s of the end of "stop" by the wall clock

#### Scenario: Stop opening a sentence is still no command
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "stop the car" without a pause
- **THEN** the service reports no command

#### Scenario: Press enter after a sentence is reported soon after the pause
- **GIVEN** the command spotting service runs with the streaming recognizer on an otherwise idle machine
- **WHEN** an app streams a sentence ending with "enter", followed by silence, in real time
- **THEN** the service reports "press enter" within 0.6 s of the end of "enter" by the wall clock

#### Scenario: The user's recorded commands are reported within their bound
- **GIVEN** the user's own recordings of the commands exist on the machine
- **WHEN** each recorded "start" and "stop" is streamed to the service in real time
- **THEN** at least half are reported within 0.3 s and at least 9 of 10 within 0.6 s of the end of their words

### Requirement: Streaming commands keep their bounds while every core is busy
With the streaming recognizer and every core of the machine busy with other work, the command spotting service SHALL report "press enter" said as "enter" ending a sentence within 1 s of the end of its words (wall clock); on an idle machine the bound is 0.6 s.

#### Scenario: Press enter after a sentence is reported in time with every core busy
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** every core of the machine is busy with other work
- **WHEN** an app streams a sentence ending with "enter", followed by silence, in real time
- **THEN** the service reports "press enter" within 1 s of the end of "enter" by the wall clock

## REMOVED Requirements

### Requirement: "clear text" erases the focused text too
**Reason**: "clear text" is a long form, now ordinary speech; only "clear" (or "clear clear") clears the focused text.
**Migration**: Say "clear" instead of "clear text".
