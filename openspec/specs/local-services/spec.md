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
