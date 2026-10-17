## ADDED Requirements

### Requirement: Voice commands are confirmed by a speech recognition service of their own
The service scripts SHALL start, beside the speech recognition service for dictation, a speech recognition service for commands that recognizes speech with the Whisper small model, and the command spotting service they start SHALL send every confirmation to it, so that the larger dictation model never slows down the confirmation of a voice command.

#### Scenario: A command is confirmed by the speech recognition service for commands
- **GIVEN** the models that the service scripts download are on disk
- **AND** the user started all services with the service scripts
- **WHEN** an app streams "press enter" to the command spotting service
- **THEN** the command spotting service reports "press enter"
- **AND** the speech recognition service for commands answered its confirmation with the small model, and the speech recognition service for dictation got no request

## MODIFIED Requirements

### Requirement: The speech recognition service runs as several copies behind one address
The service scripts SHALL start the speech recognition service for dictation as several copies, three unless the user sets another number, behind the one address the app uses; a request SHALL go to a copy that is free, and SHALL wait while every copy is busy.

#### Scenario: Requests that arrive at once are shared by the copies
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies
- **WHEN** several transcription requests arrive at once
- **THEN** every request gets its transcript, and each of the three copies transcribed some of them

### Requirement: Short speech is transcribed ahead of long speech
While requests wait for a free copy, the speech recognition service for dictation SHALL take a request with short speech — such as a short dictation — before the requests with long speech that wait.

#### Scenario: A short utterance overtakes long speech that waits
- **GIVEN** the speech recognition service for dictation runs as two copies, both busy with long speech, and more long speech waits
- **WHEN** a short utterance arrives
- **THEN** the short utterance is transcribed before the long speech that waited

### Requirement: The service scripts start the services named
The service scripts SHALL start only the services the user names, and all of them when none is named.

#### Scenario: Only the speech recognition service is started
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the user starts only the speech recognition service for dictation with the service scripts
- **THEN** the speech recognition service for dictation answers and nothing answers on the ports of the speech recognition service for commands, the text refinement service and the command spotting service

### Requirement: A service that stops answering is started again
The service scripts SHALL check every few seconds that each service they keep running still answers — the command spotting service on its connection, the text refinement service on its health check — and SHALL stop and start again a service that fails several checks in a row, writing the restart and its reason to that service's log.

#### Scenario: The command spotting service hangs
- **GIVEN** the service scripts started the command spotting service
- **WHEN** its process stops answering without exiting
- **THEN** the service answers on its port again within 30 seconds, and its log notes that it was restarted because it stopped answering

#### Scenario: A busy service is not restarted
- **GIVEN** the service scripts started the command spotting service
- **WHEN** an app streams speech to it for a minute while the speech recognition service for commands answers its confirmations slowly
- **THEN** the service is not restarted

### Requirement: The command spotting service answers while its confirmations are slow
The command spotting service SHALL keep taking in audio, spotting commands and answering on its connection while a confirmation by the speech recognition service for commands is slow, failed or hung; a confirmation that is not answered within a few seconds SHALL be given up and logged, never ending the connection.

#### Scenario: One confirmation hangs
- **GIVEN** an app streams speech to the command spotting service
- **AND** the speech recognition service for commands never answers one confirmation
- **WHEN** the app keeps streaming for 15 seconds
- **THEN** the connection stays open and answers the app throughout, and the hung confirmation is logged as given up

#### Scenario: Commands are spotted after a hung confirmation
- **GIVEN** an app streams speech to the command spotting service
- **AND** the speech recognition service for commands never answered one confirmation
- **WHEN** the app streams the command "press enter" after it
- **THEN** the service reports "press enter"

### Requirement: The command spotting service does not pile up confirmations
While a confirmation of a running utterance waits for the speech recognition service for commands, the command spotting service SHALL skip the next check of that utterance instead of queueing it, and SHALL try once more the confirmation that decides whether a whole utterance is a command when its first attempt is not answered in time.

#### Scenario: Checks of a long utterance are skipped while the service is slow
- **GIVEN** the speech recognition service for commands takes 4 seconds to answer every confirmation
- **WHEN** an app streams one utterance of 10 seconds
- **THEN** at most one check of that utterance waits at any time, and the checks that fall due meanwhile are skipped

#### Scenario: A command said alone is confirmed by the second attempt
- **GIVEN** the speech recognition service for commands does not answer the first confirmation in time and answers the second
- **WHEN** an app streams "press enter" said alone
- **THEN** the service reports "press enter"

### Requirement: Speech that is no command is confirmed quickly
The speech recognition service for commands SHALL answer the command spotting service's confirmation of any utterance up to 2.5 seconds in under 2 seconds when it is otherwise idle, also for speech in another language that makes speech recognition repeat itself, so a confirmation never waits out its deadline and never holds up the commands said after it.

#### Scenario: Russian speech that makes speech recognition repeat itself
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says Russian counting or filler words ("раз, два, три, четыре", "да, да, да, вот так") that the command spotting service sends for confirmation
- **THEN** every confirmation of that speech is answered in under 2 seconds

#### Scenario: A command right after such speech is carried out
- **GIVEN** voice control runs and the speech recognition service for commands is otherwise idle
- **WHEN** the user says that Russian speech and then "press enter"
- **THEN** the command spotting service reports "press enter"
