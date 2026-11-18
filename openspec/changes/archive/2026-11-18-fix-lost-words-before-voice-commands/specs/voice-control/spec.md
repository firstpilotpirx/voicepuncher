## MODIFIED Requirements

### Requirement: A command counts wherever its word is said
The system SHALL carry out a command as soon as its word is heard, wherever it is said in an utterance — on its own, at the end of other speech, at its start, or inside a sentence with speech after it — when the state accepts the command; no pause before or after it is needed. The user keeps the command words out of dictation, so a command word inside a sentence counts as the command. Ordinary speech without a command word triggers nothing. A command word counts only when it is said as a whole word: a longer word that holds its sounds or opens with them («интернет», «линтер», «интервью» for "enter") is ordinary speech, also while the recognizer has heard only its opening so far.

#### Scenario: A longer word opening with the sounds of enter is no command
- **GIVEN** a dictation runs
- **WHEN** the user says «откуда её возьму интернет» without pausing, the recognizer first writing the last word as «интер»
- **THEN** the Return key is not pressed
- **AND** the dictation goes on

#### Scenario: A word holding the sounds of enter inside it is no command
- **GIVEN** a dictation runs
- **WHEN** the user says «должен линтер проверять» without pausing
- **THEN** the Return key is not pressed
- **AND** the dictation goes on

#### Scenario: Enter written as the opening of longer words still counts at the end of speech
- **GIVEN** a dictation runs
- **WHEN** the user says «тоже учесть энтер» and the recognizer writes the last word as «интер», then the user pauses
- **THEN** the dictation ends and «тоже учесть» is pasted
- **AND** the Return key is pressed

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
The system SHALL carry out a command whose word ends an utterance, even with speech right before it in the same utterance, when the state accepts the command, as soon as the word is heard: no pause after it is awaited, except for "enter" written as «интер» at the end of speech, which counts at the pause after it ("Streaming commands are reported as soon as their words are decoded") (a command only the second look at the end of the utterance hears comes after the close, as "A fast enter glued to the end of speech counts" states). When the command ends a dictation, the pasted text SHALL hold the speech before the command and SHALL NOT hold the command words, also when the dictation's text repeats the last words before the command after them (the last chunk heard them again). The dictation SHALL keep every word said before the command's first word, also when the audio the command spotting service hears runs behind the clock (its microphone started late, lost audio or drifted).

#### Scenario: A short phrase right before enter is pasted
- **GIVEN** a dictation runs
- **AND** the audio the command spotting service hears runs 0.8 s behind the clock
- **WHEN** the user says «да, идём дальше» and then "enter" without a pause
- **THEN** «да, идём дальше» is pasted into the focused app, not only «да»

#### Scenario: The words right before enter are kept after a long dictation
- **GIVEN** a dictation runs
- **AND** the audio the command spotting service hears runs 0.8 s behind the clock
- **WHEN** the user says a long sentence ending in «ну что нам мне это читать» and then "enter" without a pause
- **THEN** the pasted text ends with «ну что нам мне это читать»

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

#### Scenario: A command word followed by a repeat of the words before it is not pasted
- **GIVEN** a dictation ended by "enter"
- **AND** its merged text is "Ее типа нет, что ли, вообще? Enter! вообще."
- **WHEN** its text is pasted
- **THEN** the pasted text is "Ее типа нет, что ли, вообще?"

### Requirement: Streaming commands are reported as soon as their words are decoded
On an idle machine the service SHALL report "start" and "stop" within 0.3 s of their words' end (wall clock) for half and within 0.6 s for 9 of 10. A command word SHALL count once a recognizer decoded it, wherever it sits, without a pause, unless it may grow into a longer command word ("start" of "restart") or it is the last word heard so far and the Russian recognizer wrote "enter" as «интер», the opening of many ordinary words («интернет», «интервью»): such a word counts once a next word follows it, at the gate's pause after it, or when the utterance closes. During a dictation, a one-word command only the English recognizer hears, where the Russian one heard another word, SHALL count only at the pause after it; doubled forms never wait.

#### Scenario: Start said alone is reported within its bound
- **GIVEN** the command spotting service runs with the streaming recognizer on an otherwise idle machine
- **WHEN** an app streams "start" followed by silence, in real time, five times
- **THEN** the service reports "start dictation" within 0.3 s of the end of "start" by the wall clock in at least three of them, and within 0.6 s in all five

#### Scenario: Stop said alone is reported at once
- **GIVEN** the command spotting service runs with the streaming recognizer on an otherwise idle machine
- **WHEN** an app streams "stop" followed by silence, in real time, five times
- **THEN** the service reports "stop dictation" within 0.4 s of the end of "stop" by the wall clock in at least three of them, and within 0.6 s in all five

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

#### Scenario: Enter written as «интер» at the end of speech counts at the pause
- **GIVEN** a dictation runs
- **WHEN** the Russian recognizer writes the last word of the user's speech as «интер» and the gate hears a pause right after it
- **THEN** "press enter" is reported once the recognizer decoded up to that pause, within 0.6 s of the end of the word
