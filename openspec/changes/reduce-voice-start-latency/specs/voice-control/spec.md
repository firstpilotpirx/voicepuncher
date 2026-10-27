## ADDED Requirements

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
- **AND** the sentence after "start start" is part of the dictation

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

## RENAMED Requirements

- FROM: `### Requirement: A voice start gives its start cue within 0.7 s`
- TO: `### Requirement: A voice start gives its start cue within 0.5 s`
- FROM: `### Requirement: An utterance ends after 0.3 s of silence`
- TO: `### Requirement: An utterance ends after 0.2 s of silence`

## MODIFIED Requirements

### Requirement: A voice start gives its start cue within 0.5 s
When a voice command starts a dictation from standby, or "restart" starts one over, the system SHALL play the sound that start plays (the restart sound for "restart") and show the overlay's recording sign as soon as the command is confirmed, without waiting for the microphone of the dictation to open, within 0.5 s of the end of the command's words in at least 9 of 10 starts, with the services otherwise idle; a single-word "start" or "restart" followed at once by speech, which is heard as a command only once the next word has begun, SHALL give it within 0.7 s. In the scenarios below "the start cue" means that sound. This holds for "start" alone, opening or ending an utterance, or doubled, with "please" after it, and every form of "restart", also when the first hearing of a quiet "restart" takes it for another word. A check of this requirement SHALL time the start cue only on an idle machine: it waits for the builds, tests and model stages of every other check on the machine to end and keeps new ones out while it times, so the bound is measured under the condition it states. A start cue over its bound SHALL be reported with what ran beside it: the busiest processes on the machine and every check that held a lock at that moment. A check SHALL time the start cue only against warm copies of the speech recognition service for commands: each copy has transcribed speech since it started, so no confirmation of a timed start waits for a copy's first decode. A start cue that never came because the confirmation of the command was cut short SHALL be reported with the copies whose confirmation was cut short, after how long each was cut short, and what ran beside them, as a late cue is.

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
- **THEN** the start cue comes within 0.5 s of the end of "start"

#### Scenario: Restart restart followed by the new dictation at once gives its cue at once
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "restart restart" and another sentence without a pause
- **THEN** the start cue of the new dictation comes within 0.5 s of the end of "restart restart"

#### Scenario: Restart said on its own gives its cue at once
- **GIVEN** a dictation runs
- **AND** the user has dictated for about 20 s
- **WHEN** the user says "restart" and stays silent
- **THEN** the start cue of the new dictation comes within 0.5 s of the end of "restart"

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
- **THEN** the start cue comes within 0.5 s of the end of "start please", while the sentence is still being said

#### Scenario: Start please said on its own gives its cue at once
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start please" and stays silent
- **THEN** the start cue comes within 0.5 s of the end of "start please"

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

### Requirement: An utterance ends after 0.2 s of silence
The system SHALL end an utterance after 0.2 s of silence, so a command said after a short breath is an utterance of its own, also in a noisy room where the silence is the room's own noise. This SHALL hold for a quiet voice and when speech follows the pause at once: a command said on its own, followed by 0.2–0.6 s of silence and then speech, SHALL take effect. The speech after the pause SHALL NOT change what the command is heard as. A pause is the silence between the words as said, not the silence a speech synthesizer adds after a word.

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
- **WHEN** the user says "enter", pauses for about 0.25 s and says a sentence, then pauses
- **THEN** the Return key is pressed once
- **AND** no other command takes effect

#### Scenario: The user's recorded command alone, a short pause, then speech
- **GIVEN** the user's own recordings of the commands exist on the machine
- **AND** the state accepts the command
- **WHEN** the user's recorded "restart", "start", "stop" or "enter", said on its own, is followed by 0.2–0.6 s of silence and then speech
- **THEN** the command takes effect once
- **AND** no other command takes effect

#### Scenario: A command in a noisy room ends at its pause
- **GIVEN** listening is on
- **AND** no dictation runs
- **AND** steady room noise plays
- **WHEN** the user says "start" and stays silent
- **THEN** the start cue comes within 0.5 s of the end of "start"

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
