## MODIFIED Requirements

### Requirement: Speech that is no command is confirmed quickly
The speech recognition service for commands SHALL answer the command spotting service's confirmation of any utterance up to 2.5 seconds in under 2 seconds when it is otherwise idle, also for speech in another language that makes speech recognition repeat itself, so a confirmation never waits out its deadline and never holds up the commands said after it. A confirmation the speech recognition service has not answered within 1 second of being sent SHALL be cut short: the command spotting service takes its utterance as no command, does not send it again, and the copy that held it takes the next confirmation at once.

#### Scenario: Russian speech that makes speech recognition repeat itself
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says Russian counting or filler words ("раз, два, три, четыре", "да, да, да, вот так") that the command spotting service sends for confirmation
- **THEN** every confirmation of that speech is answered in under 2 seconds

#### Scenario: A command right after such speech is carried out
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says that Russian speech and then "press enter"
- **THEN** the command spotting service reports "press enter"

#### Scenario: A confirmation still decoding after 1 second is cut short
- **GIVEN** the speech recognition service for commands runs as one copy, which keeps decoding an utterance for 1.5 seconds before it answers
- **WHEN** the command spotting service sends that utterance for confirmation
- **THEN** the confirmation ends in about 1 second with nothing heard, the utterance is not sent again, and the next confirmation is sent to that copy at once
