## ADDED Requirements

### Requirement: A voice start gives its start cue within 0.7 s of its words
When a voice command starts a dictation from standby, or "restart" starts one over, the system SHALL play the sound that start plays (the restart sound for "restart") and show the overlay's recording sign as soon as the command is confirmed, without waiting for the microphone of the dictation to open, within 0.7 s of the end of the command's words in at least 9 of 10 synthesized starts and in all but one of the user's recorded starts, with the services otherwise idle; a doubled form ("start start", "restart restart"), which is heard as a command without waiting for the pause after it or the next word, SHALL give it within 0.5 s. In the scenarios below "the start cue" means that sound. This holds for "start" alone, opening or ending an utterance, or doubled, and every form of "restart", also when the first hearing of a quiet "restart" takes it for another word. A check of this requirement SHALL time the start cue only on an idle machine: it waits for the builds, tests and model stages of every other check on the machine to end and keeps new ones out while it times, so the bound is measured under the condition it states. A start cue over its bound SHALL be reported with what ran beside it: the busiest processes on the machine and every check that held a lock at that moment. A check SHALL time the start cue only against warm copies of the speech recognition service for commands: each copy has transcribed speech since it started, so no confirmation of a timed start waits for a copy's first decode. A start cue that never came because the confirmation of the command was cut short SHALL be reported with the copies whose confirmation was cut short, after how long each was cut short, and what ran beside them, as a late cue is.

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
- **THEN** at least 5 of 6 of them give the start cue within its bound of the end of the command's words

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


### Requirement: Streaming commands keep their bounds while every core is busy
With the streaming recognizer and every core of the machine busy with other work, the command spotting service SHALL report "press enter" ending a sentence within 1 s of the end of its words (wall clock); on an idle machine the bound is 0.6 s.

#### Scenario: Press enter after a sentence is reported in time with every core busy
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** every core of the machine is busy with other work
- **WHEN** an app streams a sentence ending with "press enter", followed by silence, in real time
- **THEN** the service reports "press enter" within 1 s of the end of "press enter" by the wall clock

## REMOVED Requirements

### Requirement: A command may end with "please"
**Reason**: the user never says "please" after a command; its keyword lines, respellings and the cut of a trailing "please" only add ways to mishear.
**Migration**: say the command alone or doubled.

### Requirement: A voice start gives its start cue within 0.7 s
**Reason**: replaced by "A voice start gives its start cue within 0.7 s of its words", which drops the "start please" scenarios with the "please" form.
**Migration**: none; the bounds are the same, except that the user's recorded starts, now six once the "start please" ones are left out, may miss once (5 of 6) instead of the former 9 of 10.

## MODIFIED Requirements

### Requirement: "clear text" erases the focused text too
The system SHALL take "clear text" as the "clear all text" command.

#### Scenario: Saying clear text selects all and deletes
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "clear text"
- **THEN** Cmd+A and then Delete are pressed in the focused app


### Requirement: A whole hands-free session runs as said
The system SHALL carry out a long hands-free session — dictations ended by "stop dictation" or "cancel dictation", Return presses and clears, in any order — exactly as said: each stopped dictation pasted with its own words and without the command phrase, each cancelled one dropped, each key press done once, in order.

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


### Requirement: Every command has a one-word short form
The system SHALL take "start" as "start dictation", "stop" as "stop dictation", "cancel" as "cancel dictation", "restart" as "restart dictation", "enter" as "press enter", "clear" as "clear all text" and "paste" as "paste text", under the same rules as the full phrase, in the state that accepts the command.

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
During a dictation, the system SHALL discard what was dictated so far, paste nothing, and start a new dictation at once when the user says "restart dictation" or its short form "restart". It plays the restart sound alone, neither the cancel sound nor the start sound, and the overlay shows the restart sign for about a second, then the recording again. "restart restart" counts anywhere, like the other doubled short forms, and "restart" may end an utterance. It is in no chain: it only follows a running dictation, and nothing follows it in one breath.

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
- **WHEN** the user says "okay, press enter" without a pause, then pauses
- **THEN** the Return key is pressed in the focused app

#### Scenario: A command word inside a sentence is still no command
- **GIVEN** a dictation runs
- **WHEN** the user says "we should stop the server tonight" and pauses
- **THEN** the dictation keeps running


### Requirement: Every event of audio accepted before a sync is out before the sync is answered
When an app asks the command spotting service to confirm that every event of the audio it sent so far is out (a sync), the service SHALL answer only after it has reported every speech segment closed in that audio, also a segment whose confirmation it had to ask again — because the first hearing came back empty, cut short under load or misheard — after the sync arrived; an app that closes the connection on that answer SHALL have received every command said in that audio.

#### Scenario: A command whose first hearing is empty under load is reported before the sync is answered
- **GIVEN** voice control runs in standby and the speech recognition service for commands answers the first hearing of a segment empty and late, as under machine load, and the next one right
- **WHEN** the user says "clear text" alone and the app asks for a sync right after the speech
- **THEN** the command spotting service reports "clear all text" before it answers the sync

#### Scenario: A command whose half-pause hearing is misheard is reported before the sync is answered
- **GIVEN** voice control runs in standby and the speech recognition service for commands mishears a command word in the hearing asked at half the pause and hears it right in the hearing of the whole segment
- **WHEN** the user says "okay, clear" with a short pause after "okay" and the app asks for a sync right after the speech
- **THEN** the command spotting service reports "clear all text" before it answers the sync


### Requirement: A command said in Russian sounds counts as the command
With the streaming recognizer, a command word the Russian recognizer hears in Cyrillic ("стоп", "энтер", "клиар", "старт dictation" heard as "старт диктейшн") SHALL count as the English command it sounds like, with the other words of the command's forms heard the same way.

#### Scenario: A Russian-accented stop is heard as stop
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "стоп" said by a Russian voice followed by silence
- **THEN** the service reports "stop dictation"

#### Scenario: A Russian word that is no command stays speech
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "привет" said by a Russian voice followed by silence
- **THEN** the service reports no command

