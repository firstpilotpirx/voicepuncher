## ADDED Requirements

### Requirement: A command spotting service that answers late does not stop listening
The system SHALL keep listening while the command spotting service answers late, however long its spotting results take, as long as the service still answers on its connection; only a dropped connection or a service that answers nothing at all for 5 seconds after the app asked SHALL count as the service going away. A pause of the app's own, during which it reads nothing from the service, SHALL NOT count as the service's silence.

#### Scenario: A pause of the app's own does not lose the service
- **GIVEN** listening is on and the command spotting service answers its pings
- **WHEN** the app reads nothing from the service for longer than its silence limit, as when it is busy or the computer sleeps
- **THEN** the app's next exchange with the service succeeds, so the service is not reported lost

#### Scenario: A burst of audio after long listening does not lose the service
- **GIVEN** listening has been on for a long time with nothing said
- **WHEN** the user says "stop" with a pause before and after it
- **THEN** the service is not reported lost
- **AND** "stop dictation" takes effect

### Requirement: Every voice start logs the time of its start cue
For every voice start in scope of the start cue bound, the system SHALL log one line with the time from the end of the command's words to the start cue and the time of each stage on the way: hearing the command, delivering it to the app, starting the cue sound and drawing the overlay. The time SHALL stay right after part of the microphone audio never reached the command spotting service.

#### Scenario: A voice start logs its timing
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start start" and a sentence without a pause
- **THEN** the log shows one timing line for that start, with its total and the time of each stage, the cue sound and the overlay included

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

### Requirement: Every command in audio accepted before a sync is out before the sync is answered
When an app asks the command spotting service to confirm that every event of the audio it sent so far is out (a sync), the service SHALL answer only after both streaming recognizers have decoded that audio and every command heard in it is reported; an app that closes the connection on that answer SHALL have received every command said in that audio.

#### Scenario: A command said right before a sync is reported before the sync is answered
- **GIVEN** voice control runs in standby
- **WHEN** the user says "enter" alone and the app asks for a sync right after the speech
- **THEN** the command spotting service reports "press enter" before it answers the sync

### Requirement: A voice start gives its start cue within 0.7 s of the end of its words
When a voice command starts a dictation from standby, or "restart" starts one over, the system SHALL play its start cue (the restart sound for "restart") and show the overlay's recording sign as soon as the command is heard, within 0.7 s of the end of the command's words in at least 9 of 10 synthesized starts and in all but one of the user's recorded starts, with the services otherwise idle; a doubled form ("start start", "restart restart") SHALL give it within 0.5 s.
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
- **WHEN** the user says a sentence with "start" inside it, ordinary speech or a short Russian phrase
- **THEN** no dictation starts

### Requirement: The start cue is timed on an idle machine
A check of the start cue bound SHALL time the start cue only once the builds, tests and model stages of every other check on the machine have ended, keeping new ones out while it times; a start cue over its bound SHALL be reported with the busiest processes on the machine and every check that held a lock at that moment.

#### Scenario: The start cue is timed once the other checks' builds have ended
- **GIVEN** another check on the machine is building or testing while this check reaches its cue-timing scenarios
- **WHEN** the cue-timing scenarios are due
- **THEN** they wait for that build or test to end, no new build or test of another check starts beside them, and they time the start cue only then

#### Scenario: A late start cue names what ran beside it
- **GIVEN** the start cue of a voice start came later than its bound
- **WHEN** the failure is reported
- **THEN** the report names the busiest processes on the machine and every check that held a lock at that moment

## MODIFIED Requirements

### Requirement: Streaming recognizers hear commands behind a voice gate
The command spotting service SHALL run a voice gate over the audio and, from the onset of speech, feed an English and a Russian streaming recognizer the speech together with the half second before the onset, until the speech has ended. A command either recognizer hears SHALL be reported as soon as it is heard, without a Whisper confirmation, and without waiting for the other recognizer to decode the same audio. Without speech, no recognizer SHALL run. The gate SHALL open on a command said as quietly as the quietest commands in the user's own recordings.

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
- **THEN** the start cue comes within 0.9 s of the end of "start"

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
- **THEN** the start cue comes within 0.6 s of the end of "start start", while the sentence is still being said

### Requirement: Streaming commands keep their bounds while every core is busy
With the streaming recognizer and every core of the machine busy with other work, the command spotting service SHALL report "press enter" said as "enter" ending a sentence within 1.2 s of the end of its words (wall clock); on an idle machine the bound is 0.6 s.

#### Scenario: Press enter after a sentence is reported in time with every core busy
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** every core of the machine is busy with other work
- **WHEN** an app streams a sentence ending with "enter", followed by silence, in real time
- **THEN** the service reports "press enter" within 1.2 s of the end of "enter" by the wall clock

## REMOVED Requirements

### Requirement: A command said twice in one utterance counts once
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: Confirmations are not steered by a prompt
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: A command is carried out once however fast its confirmation comes back
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: A confirmation never waits past its deadline for a copy
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: A confirmation refused by its copy goes to another copy
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: A confirmation whose copy stalls goes to another copy
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: A command whose confirmation fails is reported
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: Every check due for a spotted "start" is asked
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: A command confirmed slowly under load is still carried out
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: Every utterance with a command heard is kept in the command history
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: Commands can run without the Whisper confirmation
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: A command carried out without confirmation records whether Whisper agreed
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: A slow command spotting service does not stop listening
**Reason**: its scenarios of the keyword spotter's confirmation are gone with it.
**Migration**: replaced by "A command spotting service that answers late does not stop listening".

### Requirement: Every voice start logs how long its start cue took
**Reason**: its scenarios of the keyword spotter's confirmation are gone with it.
**Migration**: replaced by "Every voice start logs the time of its start cue".

### Requirement: Every event of audio accepted before a sync is out before the sync is answered
**Reason**: its scenarios of the keyword spotter's confirmation are gone with it.
**Migration**: replaced by "Every command in audio accepted before a sync is out before the sync is answered".

### Requirement: A voice start gives its start cue within 0.7 s of its words
**Reason**: its scenarios of the keyword spotter's confirmation are gone with it.
**Migration**: replaced by "A voice start gives its start cue within 0.7 s of the end of its words".
