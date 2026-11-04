## MODIFIED Requirements

### Requirement: The user's own way of saying the commands is recognized
The system SHALL recognize the commands as the user says them, including small slips of pronunciation or of the speech recognition service ("and restart" for a quick "enter start" closing a chain). A command said on its own SHALL be recognized whatever the length of the pause before it, and in a quiet voice, as quiet as the user's quietest recorded commands, as well as at a normal level. A quiet command word the voice gate stops hearing before the recognizers have decoded it SHALL still be decoded at the end of its utterance: the service gives every recognizer the pause after the word before their final decode. Every command the user said in a recording kept for testing SHALL be recognized exactly once, apart from recordings marked unclear; repeats said without a pause of 0.3 s are one command. A short Russian phrase SHALL NOT be taken for a command; "стоп" is left out of this rule, since the speech recognition service may hear it as the short form "stop".

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

#### Scenario: The user's quiet start cut short by the gate is recognized
- **GIVEN** the user's own recordings of the commands exist on the machine, among them a quiet "старт" said after a breath, and the moments the service's voice gate heard it at the time: its last speech 0.4 s before the end of the utterance, before the word's last sound
- **AND** no dictation runs
- **WHEN** the recording is heard with the gate hearing it as it did at the time
- **THEN** a dictation starts once, at the "старт"

### Requirement: Streaming commands are reported within 0.3 s of their words
With the streaming recognizer on an idle machine, the command spotting service SHALL report "start" and "stop" within 0.3 s of the end of their words (wall clock) for at least half of them and within 0.6 s for 9 of 10; a command said alone or ending a sentence SHALL count at the pause the gate hears after it, without waiting for the utterance to close, unless its last word is also the end of a longer command word the state listens for ("start" in "restart"), which counts at the close. A one-word command a recognizer heard while the gate still hears the speech going on SHALL NOT count yet, however much audio the recognizer decoded past it: it counts once the gate hears a pause begin right after it, at a sync the app asks for after it (for a command said alone the sync stands for the pause, so the command is reported before the sync is answered; a command word ending the words heard so far does not count at a sync, which the app asks for inside a sentence too), or at the close of the utterance; a Russian word a recognizer hears as "stop" at the head of a sentence that goes on is no command.

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

#### Scenario: A Russian sentence heard as opening with stop is no command
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** the user's own recordings exist on the machine, among them "чтобы зайти в гетлаб" said while dictating, whose first word the recognizer hears as "stop"
- **WHEN** an app streams that recording in real time, listening for the commands of a dictation
- **THEN** the service reports no command

#### Scenario: Press enter after a sentence is reported soon after the pause
- **GIVEN** the command spotting service runs with the streaming recognizer on an otherwise idle machine
- **WHEN** an app streams a sentence ending with "enter", followed by silence, in real time
- **THEN** the service reports "press enter" within 0.6 s of the end of "enter" by the wall clock

#### Scenario: The user's recorded commands are reported within their bound
- **GIVEN** the user's own recordings of the commands exist on the machine
- **WHEN** each recorded "start" and "stop" is streamed to the service in real time
- **THEN** at least half are reported within 0.3 s and at least 9 of 10 within 0.6 s of the end of their words

### Requirement: A doubled command needs no pause
The system SHALL carry out a doubled short form ("start start", "restart restart", "stop stop", "cancel cancel", "enter enter", "clear clear", "paste paste") as soon as its second word is heard, without waiting for a pause after it or for the next word, whether silence or speech follows. Inside running speech the second word is heard once the recognizer emits it, which takes the speech past it (the first word alone fires nothing while the gate hears speech): a doubled form said inside a sentence SHALL take effect within 1 s of its end, before the sentence ends. A doubled one-word form SHALL end with its first word: a recognizer that emits late times the second word inside the speech said after it, so the dictation a doubled start or restart begins runs from the end of the first word and the second word is removed from its text.

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
