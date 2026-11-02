## MODIFIED Requirements

### Requirement: The command spotting service answers while its confirmations are slow
The command spotting service SHALL keep taking in audio, spotting commands and answering on its connection while a confirmation by the speech recognition service for commands is slow, failed or hung; a confirmation that is not answered within a few seconds SHALL be given up and logged, never ending the connection. How long the service waits for a confirmation SHALL be settable when it starts, so a test proves the give-up without waiting the default.

#### Scenario: One confirmation hangs
- **GIVEN** an app streams speech to the command spotting service
- **AND** the speech recognition service for commands never answers one confirmation
- **WHEN** the app keeps streaming past the wait for that confirmation
- **THEN** the connection stays open and answers the app throughout, and the hung confirmation is logged as given up

#### Scenario: Commands are spotted after a hung confirmation
- **GIVEN** an app streams speech to the command spotting service
- **AND** the speech recognition service for commands never answered one confirmation
- **WHEN** the app streams the command "enter" after it
- **THEN** the service reports "press enter"

### Requirement: The command spotting service does not pile up confirmations
While a confirmation of a running utterance waits for the speech recognition service for commands, the command spotting service SHALL skip the next check of that utterance instead of queueing it, and SHALL try once more the confirmation that decides whether a whole utterance is a command when its first attempt is not answered in time.

#### Scenario: Checks of a long utterance are skipped while the service is slow
- **GIVEN** the speech recognition service for commands takes 4 seconds to answer every confirmation
- **WHEN** an app streams one utterance of 10 seconds
- **THEN** at most one check of that utterance waits at any time, and the checks that fall due meanwhile are skipped

#### Scenario: A command said alone is confirmed by the second attempt
- **GIVEN** the speech recognition service for commands does not answer the first confirmation in time and answers the second
- **WHEN** an app streams "enter" said alone
- **THEN** the service reports "press enter"

### Requirement: Speech that is no command is confirmed quickly
The speech recognition service for commands SHALL answer the command spotting service's confirmation of any utterance up to 2.5 seconds in under 2 seconds when it is otherwise idle, also for speech in another language that makes speech recognition repeat itself, so a confirmation never waits out its deadline and never holds up the commands said after it. A confirmation the speech recognition service has not answered within its decoding budget SHALL be cut short: the command spotting service takes its utterance as no command, does not send it again, and the copy that held it takes the next confirmation at once. The decoding budget SHALL follow the pace of the service's recent answers to ordinary speech: 1 second when they came back quickly, longer when the last few took long, as under machine load, and never past the confirmation's deadline, so speech that decodes slowly because the machine is busy is heard in full while speech that decodes long because speech recognition repeats itself is still cut short. A confirmation cut short SHALL lengthen the budget to 1.5 seconds and no further, however many are cut short and whatever budget they were cut at, and an answer of speech recognition stuck repeating SHALL NOT lengthen the budget, so speech stuck repeating never raises the budget past the time a confirmation of it may take.

#### Scenario: Russian speech that makes speech recognition repeat itself
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says Russian counting or filler words ("раз, два, три, четыре", "да, да, да, вот так") that the command spotting service sends for confirmation
- **THEN** every confirmation of that speech is answered in under 2 seconds

#### Scenario: A command right after such speech is carried out
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says that Russian speech and then "enter"
- **THEN** the command spotting service reports "press enter"

#### Scenario: A confirmation still decoding after 1 second is cut short
- **GIVEN** the speech recognition service for commands runs as one copy, which answered its recent confirmations quickly and keeps decoding an utterance for 1.5 seconds before it answers
- **WHEN** the command spotting service sends that utterance for confirmation
- **THEN** the confirmation ends in about 1 second with nothing heard, the utterance is not sent again, and the next confirmation is sent to that copy at once

#### Scenario: A confirmation decoding slowly under load is heard in full
- **GIVEN** the speech recognition service for commands runs as one copy, which took close to a second for each of its recent confirmations, as under machine load, and keeps decoding "enter" for 1.5 seconds before it answers
- **WHEN** the command spotting service sends that utterance for confirmation
- **THEN** the confirmation is answered with "enter", not cut short

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

### Requirement: Voice commands are confirmed by a speech recognition service of their own
The service scripts SHALL start, beside the speech recognition service for dictation, a speech recognition service for commands that recognizes speech with the Whisper small model, and the command spotting service they start SHALL send every confirmation to it, so that the larger dictation model never slows down the confirmation of a voice command.

#### Scenario: A command is confirmed by the speech recognition service for commands
- **GIVEN** the models that the service scripts download are on disk
- **AND** the user started all services with the service scripts
- **WHEN** an app streams "enter" to the command spotting service
- **THEN** the command spotting service reports "press enter"
- **AND** the speech recognition service for commands answered its confirmation with the small model, and the speech recognition service for dictation got no request
