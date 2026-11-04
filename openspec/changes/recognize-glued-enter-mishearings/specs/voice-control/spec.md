# Spec Delta

## MODIFIED Requirements

### Requirement: A command said in Russian sounds counts as the command
With the streaming recognizer, a command word the Russian recognizer hears in Cyrillic ("стоп", "энтер", "клиар", "старт") SHALL count as the English command it sounds like. A word the Russian recognizer writes for "enter" said twice, glued into one word ("интернтер"), SHALL count as "enter enter": a doubled command, anywhere in an utterance, with no pause needed; a mangled word it writes for one "enter" ("нтр") SHALL count as "enter", in the places "enter" counts. These words are a fixed list from the user's command history, each a whole word, so an ordinary Russian word that holds the same letters ("интересно", "центр") SHALL stay speech.

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
- **GIVEN** the words the Russian recognizer wrote for "enter" and "enter enter" in the user's command history
- **WHEN** each word is read as the Russian recognizer's
- **THEN** each glued double reads as "enter enter", the glued triple too, and each mangled single reads as "enter"

#### Scenario: An ordinary Russian word with the letters of enter stays speech
- **GIVEN** the ordinary Russian words that hold the letters of "enter" ("интересно", "интерфейса", "интервью", "центр", "контракт", "контроллер", "линтры")
- **WHEN** each word is read as the Russian recognizer's
- **THEN** each stays the word it is and is no command
