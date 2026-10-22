# Spec Delta

## MODIFIED Requirements

### Requirement: Speech that is no command is confirmed quickly
The speech recognition service for commands SHALL answer the command spotting service's confirmation of any utterance up to 2.5 seconds in under 2 seconds when it is otherwise idle, also for speech in another language that makes speech recognition repeat itself, so a confirmation never waits out its deadline and never holds up the commands said after it. A confirmation the speech recognition service has not answered within its decoding budget SHALL be cut short: the command spotting service takes its utterance as no command, does not send it again, and the copy that held it takes the next confirmation at once. The decoding budget SHALL follow the pace of the service's recent confirmations: 1 second when they came back quickly, longer when the last few took long, as under machine load, and never past the confirmation's deadline, so speech that decodes slowly because the machine is busy is heard in full while speech that decodes long because speech recognition repeats itself is still cut short.

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

### Requirement: Copies of a speech recognition service are found from its address
A client of a speech recognition service SHALL find the copies of the service from the service's address alone: when the service's own port accepts connections it is the one copy; otherwise every port among the four that follow its port plus ten thousand (minus ten thousand when those would pass the last port) that accepts a connection is a copy. A copy SHALL leave at its first refused connection to a request, and at a probe only when two attempts in a row get no connection, so one probe that is slow under machine load does not drop a copy that accepts connections; a copy SHALL join, again or anew, within a second of accepting connections.

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
