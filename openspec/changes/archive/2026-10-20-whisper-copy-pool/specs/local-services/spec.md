# Spec Delta

## ADDED Requirements

### Requirement: Copies of a speech recognition service are found from its address
A client of a speech recognition service SHALL find the copies of the service from the service's address alone: when the service's own port accepts connections it is the one copy; otherwise every port among the four that follow its port plus ten thousand (minus ten thousand when those would pass the last port) that accepts a connection is a copy. A copy SHALL leave at its first refused connection and join, again or anew, within a second of accepting connections.

#### Scenario: Three copies are found from the service address
- **GIVEN** the service scripts started the speech recognition service for dictation as three copies
- **WHEN** a client connects to the service by its address
- **THEN** the client sends its requests to the three copies, and nothing listens on the service's own port

#### Scenario: A single copy on the service's own port is found
- **GIVEN** the service scripts started the speech recognition service for dictation as one copy
- **WHEN** a client connects to the service by its address
- **THEN** the client sends its requests to that copy on the service's own port

#### Scenario: A copy that came back is used again
- **GIVEN** a client of the speech recognition service, and one copy stopped accepting connections
- **WHEN** that copy accepts connections again
- **THEN** within a second the client sends requests to it again

### Requirement: A request whose copy died is retried on another copy
When a request cannot connect to the copy it took, the client SHALL mark that copy down and send the next attempt of the request to another copy that is up, within the request's own limits on attempts and time; when no attempt reaches a copy, the request SHALL fail naming the copies it tried.

#### Scenario: A copy dies as a request reaches it
- **GIVEN** the speech recognition service runs as several copies
- **AND** one copy exits as a request is about to reach it
- **WHEN** the request is sent
- **THEN** the request is answered by another copy, and the failed attempt on the dead copy is logged

#### Scenario: Every copy refuses the request
- **GIVEN** the speech recognition service runs as several copies, and none accepts connections
- **WHEN** a request is sent
- **THEN** the request fails once its attempts are used up, naming the copies it tried

### Requirement: A copy that stops answering is reported hung
A copy that has not answered a request within twice that request's time limit SHALL be logged as hung, naming the copy and the time waited, and SHALL get no new request until it accepts a connection and answers again.

#### Scenario: A copy never answers a request
- **GIVEN** the speech recognition service runs as several copies, and one copy stops answering without exiting
- **WHEN** a request waits on it past twice its time limit
- **THEN** that copy is logged as hung with the time waited, and the following requests go to the other copies

### Requirement: Clients of one process share the copies
Every client of one speech recognition service within one process SHALL share one view of its copies and one queue, and each process SHALL start its search for a free copy at a copy picked by its process id, so that two processes usually take different copies and requests sent at once from several clients or processes are spread over the copies rather than piled on the first one.

#### Scenario: Nine requests from one process spread over three copies
- **GIVEN** the speech recognition service for dictation runs as three copies
- **WHEN** nine clients of one process send a request each at once
- **THEN** each copy transcribes some of the requests, and no copy gets a second request before it answered the first

#### Scenario: Two processes start with different copies
- **GIVEN** the speech recognition service for dictation runs as three copies and is idle
- **AND** two processes whose process ids pick different copies
- **WHEN** each sends a request
- **THEN** the requests go to two different copies

## RENAMED Requirements

- FROM: `### Requirement: The speech recognition service runs as several copies behind one address`
- TO: `### Requirement: The speech recognition service runs as several copies found from its address`

## MODIFIED Requirements

### Requirement: The speech recognition service runs as several copies found from its address
The service scripts SHALL start the speech recognition service for dictation as several copies, three unless the user sets another number, on the ports a client finds from the service's address; a request SHALL go to a copy that is free, SHALL wait its turn while every copy is busy, first come first served, and no copy SHALL get two requests at once.

#### Scenario: Requests that arrive at once are shared by the copies
- **GIVEN** the service scripts started the speech recognition service for dictation without a number of copies
- **WHEN** several transcription requests arrive at once
- **THEN** every request gets its transcript, and each of the three copies transcribed some of them

#### Scenario: Requests wait their turn when every copy is busy
- **GIVEN** the speech recognition service for dictation runs as two copies, both busy with long speech
- **WHEN** two more requests arrive one after another
- **THEN** each starts on the first copy that frees, in the order they arrived, and no copy transcribes two requests at once

### Requirement: Stopping the speech recognition service stops every copy
The service scripts SHALL stop all copies of the speech recognition service, so that nothing of it keeps running.

#### Scenario: The user stops the speech recognition service that runs as several copies
- **GIVEN** the service scripts started the speech recognition service as several copies
- **WHEN** the user stops it with the service scripts
- **THEN** no copy of it is left, and nothing answers on any of its ports

### Requirement: Several copies that cannot start are reported and leave nothing behind
When a copy of the speech recognition service cannot start, the service scripts SHALL report the failure with that copy's log and SHALL stop the copies they already started.

#### Scenario: The second of several copies cannot start
- **GIVEN** the speech recognition service is to run as several copies, and its second copy cannot start
- **WHEN** the service scripts start the speech recognition service
- **THEN** the start fails naming the service and the log of the copy that failed, and the copy that started is not left

### Requirement: A request given up by its client does not hold a copy
A client that gives up on a request SHALL leave its copy alone until the copy answers that request: the copy SHALL get no new request meanwhile, and the client's next request SHALL go to another copy or wait, so an abandoned request never delays the request after it on the same copy.

#### Scenario: The request after an abandoned one goes to another copy
- **GIVEN** the speech recognition service runs as two copies
- **AND** a client gave up on a request that one copy is still transcribing
- **WHEN** the client sends its next request
- **THEN** the other copy answers it, and the first copy gets a new request only after it answered the abandoned one

#### Scenario: An abandoned request is not transcribed
- **GIVEN** the speech recognition service runs as two copies, both busy with long speech
- **AND** a third request waits and its client gives up
- **WHEN** the copies become free
- **THEN** the abandoned request is not sent to any copy

## REMOVED Requirements

### Requirement: Short speech is transcribed ahead of long speech
**Reason**: Since voice commands are confirmed by a speech recognition service of their own, no short confirmation shares a queue with long dictation chunks; the priority was dead configuration of the removed proxy.
**Migration**: None: a short request still takes the first free copy of its own service.
