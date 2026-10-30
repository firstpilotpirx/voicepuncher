## ADDED Requirements

### Requirement: Streaming recognizers hear commands behind a voice gate
With the streaming recognizer, the command spotting service SHALL run a voice gate over the audio and, from the onset of speech, feed an English and a Russian streaming recognizer the speech together with the half second before the onset, until the speech has ended. A command either recognizer hears SHALL be reported as soon as it is heard, without a Whisper confirmation. Without speech, no recognizer SHALL run.

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

### Requirement: A command said in Russian sounds counts as the command
With the streaming recognizer, a command word the Russian recognizer hears in Cyrillic ("стоп", "энтер", "клиар", "старт dictation" heard as "старт диктейшн") SHALL count as the English command it sounds like, with "please" and the other words of the command's forms heard the same way.

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
- **WHEN** an app streams "press enter" followed by silence
- **THEN** the service reports "press enter" exactly once

#### Scenario: A doubled command is reported once
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "enter enter" followed by silence
- **THEN** the service reports "press enter" exactly once

### Requirement: Each streaming utterance is kept with its texts and stage times
With the streaming recognizer and a command history, the service SHALL keep every utterance in which a recognizer heard a command word: its audio from the replayed half second on, each recognizer's texts with their stream times, the command and the recognizer that heard it, the times of onset, first text, report and end of speech, and each recognizer's decode time; and SHALL log one line per utterance with those times.

#### Scenario: A reported command leaves an entry with its timeline
- **GIVEN** the command spotting service runs with the streaming recognizer and keeps a command history
- **WHEN** an app streams "press enter" followed by silence
- **THEN** the command history holds an entry with the utterance's audio
- **AND** the entry names "press enter", the recognizer that heard it, each recognizer's texts with their times and the times of each stage
