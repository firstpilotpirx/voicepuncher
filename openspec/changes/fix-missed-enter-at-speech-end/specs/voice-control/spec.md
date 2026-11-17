## ADDED Requirements

### Requirement: A fast enter glued to the end of speech counts
With the streaming recognizer, a fast "enter" glued to the last word of an utterance SHALL be reported as "press enter", also when the running decode missed it. An utterance that closes with no reported command SHALL have its last second of speech decoded again, alone; a command heard there SHALL be reported after the close, within 1.5 s of the end of its words on an idle machine. An utterance that reported a command SHALL report nothing more. Both recognizers SHALL be biased towards "enter".

#### Scenario: A fast enter enter glued to the last word presses Return
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** no dictation runs
- **WHEN** the app streams the user's recorded sentence that ends in "enter, enter" said within half a second, glued to its last word, followed by silence
- **THEN** the service reports "press enter" exactly once

#### Scenario: Every enter the running decode missed in the user's history is heard
- **GIVEN** the user's recorded utterances of 2026-11-16 and 2026-11-17 whose last word was a fast "enter" that the service did not report
- **WHEN** each is streamed through the service, followed by silence
- **THEN** the service reports "press enter" for each

#### Scenario: Speech without a command ending it stays speech in the second look
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** the app streams a Russian sentence that ends in an ordinary word ("интересно"), followed by silence
- **THEN** the service reports no command

#### Scenario: An utterance that reported its command reports it once
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** the app streams "okay, enter" followed by silence, and the running decode reports "press enter"
- **THEN** the service reports "press enter" exactly once

## MODIFIED Requirements

### Requirement: A command said in Russian sounds counts as the command
With the streaming recognizer, a command word the Russian recognizer hears in Cyrillic ("стоп", "энтер", "клиар", "старт") SHALL count as the English command it sounds like. A word the Russian recognizer writes for "enter" said twice, glued into one word ("интернтер"), SHALL count as "enter enter": a doubled command, anywhere in an utterance, with no pause needed; a mangled word it writes for one "enter" ("нтр") SHALL count as "enter", in the places "enter" counts. These words are a fixed list from the user's command history, each a whole word, so an ordinary Russian word that holds the same letters ("интересно", "центр") SHALL stay speech. A glued or cut word for "enter" found in the history is added to the list, unless it is an ordinary Russian word ("интернета"). Beyond the list, the last word the Russian recognizer writes in an utterance SHALL count as "enter" when it starts with the sounds of "энтер" and goes on with letters no command reads ("энтерф", "энтервь"); a word starting with "интер" SHALL NOT, as Russian has many ordinary such words.

#### Scenario: A Russian-accented stop is heard as stop
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "стоп" said by a Russian voice followed by silence
- **THEN** the service reports "stop dictation"

#### Scenario: A Russian word that is no command stays speech
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "привет" said by a Russian voice followed by silence
- **THEN** the service reports no command

#### Scenario: A glued enter enter written by the Russian recognizer counts as the doubled command
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** no dictation runs
- **WHEN** the Russian recognizer writes a sentence that ends in one glued word for "enter enter", such as "интернтер", while the English recognizer hears nothing
- **THEN** the service reports "press enter" exactly once, as soon as the glued word is heard

#### Scenario: A mangled enter written by the Russian recognizer counts as enter
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** no dictation runs
- **WHEN** the Russian recognizer writes a mangled word for "enter", such as "нтр", alone, followed by silence, while the English recognizer hears nothing
- **THEN** the service reports "press enter" exactly once

#### Scenario: Every glued or mangled enter from the history reads as the command
- **GIVEN** the words the Russian recognizer wrote for "enter" and "enter enter" in the user's command history, those of 2026-11-13 to 2026-11-15 included ("энтеррендер", "интернтор", "антернтер", "китрентор", "энт")
- **WHEN** each word is read as the Russian recognizer's
- **THEN** each glued double reads as "enter enter", the glued triple too, and each mangled single reads as "enter"

#### Scenario: An ordinary Russian word with the letters of enter stays speech
- **GIVEN** the ordinary Russian words that hold the letters of "enter" ("интересно", "интерфейса", "интервью", "центр", "контракт", "контроллер", "линтры", "интернета")
- **WHEN** each word is read as the Russian recognizer's
- **THEN** each stays the word it is and is no command

#### Scenario: A last word starting with the sounds of enter counts as enter
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** no dictation runs
- **WHEN** the Russian recognizer writes a sentence whose last word starts with "энтер" and goes on with other letters, such as "там отмечать энтерф", while the English recognizer hears nothing
- **THEN** the service reports "press enter" exactly once

#### Scenario: A word starting with the sounds of enter inside speech stays speech
- **GIVEN** the Russian recognizer's text "энтерф там отмечать"
- **WHEN** it is read as the Russian recognizer's
- **THEN** "энтерф" stays the word it is and is no command

### Requirement: A command ending speech counts without a pause
The system SHALL carry out a command whose word ends an utterance, even with speech right before it in the same utterance, when the state accepts the command, as soon as the word is heard: no pause after it is awaited (a command only the second look at the end of the utterance hears comes after the close, as "A fast enter glued to the end of speech counts" states). When the command ends a dictation, the pasted text SHALL hold the speech before the command and SHALL NOT hold the command words, also when the dictation's text repeats the last words before the command after them (the last chunk heard them again).

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
