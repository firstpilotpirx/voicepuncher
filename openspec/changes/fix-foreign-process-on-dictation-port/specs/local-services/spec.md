# Spec Delta

## MODIFIED Requirements

### Requirement: Copies of a speech recognition service are found from its address
A client of a speech recognition service SHALL find the copies of the service from the service's address alone: when the service's own port answers as a speech recognition service it is the one copy; otherwise every port among the four that follow its port plus ten thousand (minus ten thousand when those would pass the last port) that answers as a speech recognition service is a copy. A port answers as a speech recognition service when its health check succeeds and it names itself a whisper-server; a program that accepts connections on a port but does not answer so SHALL never receive a request, whether it holds the service's own port or a copy's. A copy SHALL leave at its first refused connection to a request, and at a probe only when two attempts in a row get no connection, so one probe that is slow under machine load does not drop a copy that accepts connections; a copy SHALL join, again or anew, within a second of answering as a speech recognition service.

#### Scenario: Three copies are found from the service address
- **GIVEN** the service scripts started the speech recognition service for dictation as three copies
- **WHEN** a client connects to the service by its address
- **THEN** the client sends its requests to the three copies, and nothing listens on the service's own port

#### Scenario: A single copy on the service's own port is found
- **GIVEN** the service scripts started the speech recognition service for dictation as one copy
- **WHEN** a client connects to the service by its address
- **THEN** the client sends its requests to that copy on the service's own port

#### Scenario: Another program on the service's own port is passed over
- **GIVEN** the speech recognition service for dictation runs as three copies, and another program that is no speech recognition service listens on the service's own port
- **WHEN** a client transcribes a dictation through the service's address
- **THEN** the client sends its requests to the three copies, the dictation is transcribed, and the other program receives no request

#### Scenario: Another program on a copy's port is passed over
- **GIVEN** the speech recognition service for dictation runs as two copies, and another program that is no speech recognition service listens on the third copy's port
- **WHEN** a client transcribes a dictation through the service's address
- **THEN** the client sends its requests to the two copies only

#### Scenario: A copy that came back is used again
- **GIVEN** a client of the speech recognition service, and one copy stopped accepting connections
- **WHEN** that copy accepts connections again
- **THEN** within a second the client sends requests to it again

#### Scenario: A copy missed by one slow probe stays
- **GIVEN** a client of the speech recognition service running as several copies, and the machine is so busy that one probe of a copy gets no connection in time while the copy still accepts connections
- **WHEN** the client probes the copies
- **THEN** the client probes that copy once more at once, and keeps sending requests to it

#### Scenario: A copy missed by two probes in a row leaves
- **GIVEN** a client of the speech recognition service running as several copies
- **WHEN** a copy gets no connection at two probe attempts in a row
- **THEN** the client sends no request to that copy until it accepts connections again

#### Scenario: A retry after a hung request reaches the other copy
- **GIVEN** the speech recognition service for dictation runs as two copies, one of which never answers, and both stay found by the probes (a probe that misses one once keeps it, as the scenario "A copy missed by one slow probe stays" proves)
- **WHEN** a transcription request hangs on the silent copy and is retried
- **THEN** the retry is answered by the other copy, and both copies received one request

## ADDED Requirements

### Requirement: Another program on a speech recognition port is named in the log
When another program that is no speech recognition service listens on the own port of a speech recognition service, or on one of its copies' ports, the client SHALL log a warning once per port, naming the port, the program's name and its process id, as long as the program stays there.

#### Scenario: A program on the dictation port is logged once
- **GIVEN** another program that is no speech recognition service listens on the own port of the speech recognition service for dictation
- **WHEN** a client of the service transcribes two dictations
- **THEN** the log holds one warning naming that port, the program's name and its process id
