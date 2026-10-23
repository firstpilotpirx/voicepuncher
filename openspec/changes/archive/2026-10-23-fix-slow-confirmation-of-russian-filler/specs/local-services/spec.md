# Spec Delta

## MODIFIED Requirements

### Requirement: Speech that is no command is confirmed quickly
The speech recognition service for commands SHALL answer the command spotting service's confirmation of any utterance up to 2.5 seconds in under 2 seconds when it is otherwise idle, also for speech in another language that makes speech recognition repeat itself, so a confirmation never waits out its deadline and never holds up the commands said after it. A confirmation the speech recognition service has not answered within its decoding budget SHALL be cut short: the command spotting service takes its utterance as no command, does not send it again, and the copy that held it takes the next confirmation at once. The decoding budget SHALL follow the pace of the service's recent answers to ordinary speech: 1 second when they came back quickly, longer when the last few took long, as under machine load, and never past the confirmation's deadline, so speech that decodes slowly because the machine is busy is heard in full while speech that decodes long because speech recognition repeats itself is still cut short. A confirmation cut short SHALL lengthen the budget to 1.5 seconds and no further, however many are cut short and whatever budget they were cut at, and an answer of speech recognition stuck repeating SHALL NOT lengthen the budget, so speech stuck repeating never raises the budget past the time a confirmation of it may take.

#### Scenario: Russian speech that makes speech recognition repeat itself
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says Russian counting or filler words ("раз, два, три, четыре", "да, да, да, вот так") that the command spotting service sends for confirmation
- **THEN** every confirmation of that speech is answered in under 2 seconds

#### Scenario: A command right after such speech is carried out
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says that Russian speech and then "press enter"
- **THEN** the command spotting service reports "press enter"

#### Scenario: A confirmation still decoding after 1 second is cut short
- **GIVEN** the speech recognition service for commands runs as one copy, which answered its recent confirmations quickly and keeps decoding an utterance for 1.5 seconds before it answers
- **WHEN** the command spotting service sends that utterance for confirmation
- **THEN** the confirmation ends in about 1 second with nothing heard, the utterance is not sent again, and the next confirmation is sent to that copy at once

#### Scenario: A confirmation decoding slowly under load is heard in full
- **GIVEN** the speech recognition service for commands runs as one copy, which took close to a second for each of its recent confirmations, as under machine load, and keeps decoding "press enter" for 1.5 seconds before it answers
- **WHEN** the command spotting service sends that utterance for confirmation
- **THEN** the confirmation is answered with "press enter", not cut short

#### Scenario: The budget never passes the deadline
- **GIVEN** the speech recognition service for commands runs as one copy, which took longer than half the confirmation deadline for each of its recent confirmations
- **WHEN** the command spotting service sends an utterance the copy keeps decoding past the deadline
- **THEN** the confirmation ends at the deadline as a failed confirmation, not later and never as an utterance heard as no command

#### Scenario: Speech cut short does not lengthen the budget past 1.5 seconds
- **GIVEN** the speech recognition service for commands runs as one copy, which answered its recent confirmations quickly, and the last confirmation was cut short after 1 second
- **WHEN** the command spotting service sends two more utterances the copy keeps decoding for 2.5 seconds each
- **THEN** each of them is cut short after about 1.5 seconds

#### Scenario: An answer stuck repeating does not lengthen the budget
- **GIVEN** the speech recognition service for commands runs as one copy, which took close to a second for each of its recent confirmations, as under machine load, and then answered one confirmation after 1.6 seconds with speech recognition stuck repeating one word
- **WHEN** the command spotting service sends an utterance the copy keeps decoding for 2.5 seconds
- **THEN** that confirmation is cut short after about 2 seconds, the budget the ordinary answers give, not at the deadline
