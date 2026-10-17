# local-services Specification

## Purpose
TBD - created by archiving change keep-services-running. Update Purpose after archive.

## Requirements

### Requirement: A service that stops is started again
The service scripts SHALL keep each local service they started running: when the service process exits while it is meant to run, it SHALL be started again after a short pause, and the restart SHALL be written to that service's log after the output of the run that ended.

#### Scenario: The speech recognition service dies while voice control runs
- **GIVEN** the service scripts started the speech recognition service
- **WHEN** its process exits on its own
- **THEN** the service answers on its port again within 30 seconds, and its log keeps the output of the run that ended followed by a note of the restart

### Requirement: A stopped service stays stopped
The service scripts SHALL stop a service together with whatever keeps it running, so that a service the user stopped is not started again.

#### Scenario: The user stops the speech recognition service
- **GIVEN** the service scripts started the speech recognition service
- **WHEN** the user stops it with the service scripts
- **THEN** nothing answers on its port, still 5 seconds later, and no process of it is left

#### Scenario: The user stops a single speech recognition service after copies were left listed
- **GIVEN** a list of copies is left from an earlier start, and the service scripts started the speech recognition service as one copy
- **WHEN** the user stops it with the service scripts
- **THEN** nothing answers on its port, still 5 seconds later, and no process of it is left

### Requirement: A service that cannot start is reported, not retried forever
The service scripts SHALL report a service that exits before it ever became ready as a start failure that names the service and shows its last log lines, and SHALL leave no process of it behind.

#### Scenario: The speech recognition model is not a model
- **GIVEN** the speech recognition model file is not a model
- **WHEN** the service scripts start the speech recognition service
- **THEN** the start fails within 15 seconds naming the service and its log, and no process of it is left

### Requirement: The service scripts start the services named
The service scripts SHALL start only the services the user names, and all of them when none is named.

#### Scenario: Only the speech recognition service is started
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the user starts only the speech recognition service for dictation with the service scripts
- **THEN** the speech recognition service for dictation answers and nothing answers on the ports of the speech recognition service for commands, the text refinement service and the command spotting service

### Requirement: The speech recognition service runs as several copies behind one address
The service scripts SHALL start the speech recognition service for dictation as several copies, three unless the user sets another number, behind the one address the app uses; a request SHALL go to a copy that is free, and SHALL wait while every copy is busy.

#### Scenario: Requests that arrive at once are shared by the copies
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies
- **WHEN** several transcription requests arrive at once
- **THEN** every request gets its transcript, and each of the three copies transcribed some of them

### Requirement: A copy that stops does not stop the service
The speech recognition service SHALL keep answering while one of its copies stops, and the stopped copy SHALL be started again.

#### Scenario: One copy of the speech recognition service dies
- **GIVEN** the service scripts started the speech recognition service as several copies
- **WHEN** one copy exits on its own
- **THEN** the service keeps answering transcription requests, and that copy answers again within 30 seconds

### Requirement: Short speech is transcribed ahead of long speech
While requests wait for a free copy, the speech recognition service for dictation SHALL take a request with short speech — such as a short dictation — before the requests with long speech that wait.

#### Scenario: A short utterance overtakes long speech that waits
- **GIVEN** the speech recognition service for dictation runs as two copies, both busy with long speech, and more long speech waits
- **WHEN** a short utterance arrives
- **THEN** the short utterance is transcribed before the long speech that waited

### Requirement: Stopping the speech recognition service stops every copy
The service scripts SHALL stop all copies of the speech recognition service and what sits in front of them, so that nothing of it keeps running.

#### Scenario: The user stops the speech recognition service that runs as several copies
- **GIVEN** the service scripts started the speech recognition service as several copies
- **WHEN** the user stops it with the service scripts
- **THEN** nothing answers on its port, and no copy of it is left

### Requirement: Several copies that cannot start are reported and leave nothing behind
When a copy of the speech recognition service cannot start, the service scripts SHALL report the failure with that copy's log and SHALL stop the copies and whatever they already started in front of them.

#### Scenario: The second of several copies cannot start
- **GIVEN** the speech recognition service is to run as several copies, and its second copy cannot start
- **WHEN** the service scripts start the speech recognition service
- **THEN** the start fails naming the service and the log of the copy that failed, and neither the copy that started nor anything in front of it is left

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

### Requirement: A request given up by its client does not hold a copy
The speech recognition service SHALL not transcribe a request whose client gave up on it while it waited for a free copy: the copy that takes it SHALL give it up at once, so an abandoned request never holds a copy for longer than a moment.

#### Scenario: An abandoned request is not transcribed
- **GIVEN** the speech recognition service runs as two copies, both busy with long speech
- **AND** a third request waits and its client gives up
- **WHEN** the copies become free
- **THEN** the copy that takes the abandoned request gives it up at once instead of transcribing it

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

### Requirement: Voice commands are confirmed by a speech recognition service of their own
The service scripts SHALL start, beside the speech recognition service for dictation, a speech recognition service for commands that recognizes speech with the Whisper small model, and the command spotting service they start SHALL send every confirmation to it, so that the larger dictation model never slows down the confirmation of a voice command.

#### Scenario: A command is confirmed by the speech recognition service for commands
- **GIVEN** the models that the service scripts download are on disk
- **AND** the user started all services with the service scripts
- **WHEN** an app streams "press enter" to the command spotting service
- **THEN** the command spotting service reports "press enter"
- **AND** the speech recognition service for commands answered its confirmation with the small model, and the speech recognition service for dictation got no request
