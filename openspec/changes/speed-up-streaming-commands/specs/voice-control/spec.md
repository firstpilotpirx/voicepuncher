## MODIFIED Requirements

### Requirement: Streaming recognizers hear commands behind a voice gate
With the streaming recognizer, the command spotting service SHALL run a voice gate over the audio and, from the onset of speech, feed an English and a Russian streaming recognizer the speech together with the half second before the onset, until the speech has ended. A command either recognizer hears SHALL be reported as soon as it is heard, without a Whisper confirmation, and without waiting for the other recognizer to decode the same audio. Without speech, no recognizer SHALL run. The gate SHALL open on a command said as quietly as the quietest commands in the user's own recordings.

#### Scenario: A command said alone is reported before its pause ends
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "press enter" followed by half a second of silence and asks for a sync
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

### Requirement: Each streaming utterance is kept with its texts and stage times
With the streaming recognizer and a command history, the service SHALL keep every utterance the voice gate opened: its audio from the replayed half second on, each recognizer's texts with their stream times, the command and the recognizer that heard it when one was reported, the times of onset, first text, report and end of speech, and each recognizer's decode time; and SHALL log one line per utterance with those times. An utterance in which no command was reported SHALL be kept as such, so a missed command can be found in the history.

#### Scenario: A reported command leaves an entry with its timeline
- **GIVEN** the command spotting service runs with the streaming recognizer and keeps a command history
- **WHEN** an app streams "press enter" followed by silence
- **THEN** the command history holds an entry with the utterance's audio
- **AND** the entry names "press enter", the recognizer that heard it, each recognizer's texts with their times and the times of each stage

#### Scenario: An utterance without a command is kept too
- **GIVEN** the command spotting service runs with the streaming recognizer and keeps a command history
- **WHEN** an app streams the sentence "the weather is nice today" followed by silence
- **THEN** the command history holds an entry with the utterance's audio and each recognizer's texts
- **AND** the entry names no command

## ADDED Requirements

### Requirement: Streaming commands are reported within 0.3 s of their words
With the streaming recognizer on an idle machine, the command spotting service SHALL report "start" and "stop" within 0.3 s of the end of their words (wall clock) for at least half of them and within 0.6 s for 9 of 10; a command said alone SHALL count at the pause the gate hears after it, and one ending an utterance after speech within 0.6 s.

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
- **WHEN** an app streams a sentence ending with "press enter", followed by silence, in real time
- **THEN** the service reports "press enter" within 0.6 s of the end of "press enter" by the wall clock

#### Scenario: The user's recorded commands are reported within their bound
- **GIVEN** the user's own recordings of the commands exist on the machine
- **WHEN** each recorded "start" and "stop" is streamed to the service in real time
- **THEN** at least half are reported within 0.3 s and at least 9 of 10 within 0.6 s of the end of their words

### Requirement: Streaming commands are heard in the user's own voice
With the streaming recognizer, the command spotting service SHALL report at least 94 of 100 commands in the user's own recordings as the command said, counting a doubled command once, and SHALL take the mishearings found in the user's command history as forms of the command they mishear.

#### Scenario: The user's recorded commands are heard
- **GIVEN** the user's own recordings of the commands exist on the machine
- **WHEN** every recording is streamed to the service
- **THEN** at least 94 of 100 commands are reported as the command said

#### Scenario: A mishearing from the history counts as its command
- **GIVEN** the user's command history shows a recognizer hearing a command word as another word, with that recording replayed
- **WHEN** the recording is streamed to the service again
- **THEN** the service reports the command said
