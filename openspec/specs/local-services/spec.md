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
- **WHEN** the user starts only the speech recognition service with the service scripts
- **THEN** the speech recognition service answers and nothing answers on the text refinement and command spotting ports

### Requirement: The speech recognition service runs as several copies behind one address
The service scripts SHALL start the speech recognition service as several copies, three unless the user sets another number, behind the one address the app and the command spotting service use; a request SHALL go to a copy that is free, and SHALL wait while every copy is busy.

#### Scenario: Requests that arrive at once are shared by the copies
- **GIVEN** the service scripts started the speech recognition service without a number of copies
- **WHEN** several transcription requests arrive at once
- **THEN** every request gets its transcript, and each of the three copies transcribed some of them

### Requirement: A copy that stops does not stop the service
The speech recognition service SHALL keep answering while one of its copies stops, and the stopped copy SHALL be started again.

#### Scenario: One copy of the speech recognition service dies
- **GIVEN** the service scripts started the speech recognition service as several copies
- **WHEN** one copy exits on its own
- **THEN** the service keeps answering transcription requests, and that copy answers again within 30 seconds

### Requirement: Short speech is transcribed ahead of long speech
While requests wait for a free copy, the speech recognition service SHALL take a request with short speech — such as the confirmation of a voice command — before the requests with long speech that wait.

#### Scenario: A short utterance overtakes long speech that waits
- **GIVEN** the speech recognition service runs as two copies, both busy with long speech, and more long speech waits
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
- **WHEN** an app streams speech to it for a minute while its confirmations are slow
- **THEN** the service is not restarted

### Requirement: The command spotting service answers while its confirmations are slow
The command spotting service SHALL keep taking in audio, spotting commands and answering on its connection while a confirmation by the speech recognition service is slow, failed or hung; a confirmation that is not answered within a few seconds SHALL be given up and logged, never ending the connection.

#### Scenario: One confirmation hangs
- **GIVEN** an app streams speech to the command spotting service
- **AND** the speech recognition service never answers one confirmation
- **WHEN** the app keeps streaming for 15 seconds
- **THEN** the connection stays open and answers the app throughout, and the hung confirmation is logged as given up

#### Scenario: Commands are spotted after a hung confirmation
- **GIVEN** an app streams speech to the command spotting service
- **AND** the speech recognition service never answered one confirmation
- **WHEN** the app streams the command "press enter" after it
- **THEN** the service reports "press enter"

### Requirement: The command spotting service does not pile up confirmations
While a confirmation of a running utterance waits for the speech recognition service, the command spotting service SHALL skip the next check of that utterance instead of queueing it, and SHALL try once more the confirmation that decides whether a whole utterance is a command when its first attempt is not answered in time.

#### Scenario: Checks of a long utterance are skipped while the service is slow
- **GIVEN** the speech recognition service takes 4 seconds to answer every confirmation
- **WHEN** an app streams one utterance of 10 seconds
- **THEN** at most one check of that utterance waits at any time, and the checks that fall due meanwhile are skipped

#### Scenario: A command said alone is confirmed by the second attempt
- **GIVEN** the speech recognition service does not answer the first confirmation in time and answers the second
- **WHEN** an app streams "press enter" said alone
- **THEN** the service reports "press enter"

### Requirement: A request given up by its client does not hold a copy
The speech recognition service SHALL not transcribe a request whose client gave up on it while it waited for a free copy: the copy that takes it SHALL give it up at once, so an abandoned request never holds a copy for longer than a moment.

#### Scenario: An abandoned request is not transcribed
- **GIVEN** the speech recognition service runs as two copies, both busy with long speech
- **AND** a third request waits and its client gives up
- **WHEN** the copies become free
- **THEN** the copy that takes the abandoned request gives it up at once instead of transcribing it
