## ADDED Requirements

### Requirement: A hung confirmation makes the command spotting service unhealthy
The command spotting service SHALL count a confirmation that runs longer than its attempts may take, and at least 10 seconds, as hung: its health check SHALL answer unavailable naming the connection, and it SHALL tell the app on that connection, which counts the service as lost and reaches for another copy.

#### Scenario: A confirmation thread that hangs is seen
- **GIVEN** an app streams speech to the command spotting service
- **AND** one confirmation hangs inside the service past its attempts
- **WHEN** the health of the service is checked
- **THEN** it answers unavailable naming the connection, and the app was told the service stalled

#### Scenario: The app leaves a stalled service
- **GIVEN** listening is on
- **WHEN** the command spotting service reports that it stalled
- **THEN** the service is counted as lost and the app reaches for it again

### Requirement: Confirmations waiting in the command spotting service are bounded
The command spotting service SHALL keep at most a few confirmations waiting per confirmation thread of a connection while the confirmation that thread runs is stalled; a confirmation asked past that SHALL fail at once and be logged, never queue without bound. A thread that keeps answering, however slowly, SHALL queue every confirmation asked of it.

#### Scenario: A confirmation past the bound fails at once
- **GIVEN** the speech recognition service for commands answers nothing, the confirmation a thread runs has run past its attempts, and the thread already holds its bound of waiting confirmations
- **WHEN** one more confirmation is asked on that thread
- **THEN** it fails at once and the failure is logged

#### Scenario: A slow confirmation thread keeps every confirmation
- **GIVEN** the speech recognition service for commands answers every confirmation, late
- **WHEN** more confirmations than the bound are asked on one thread at once
- **THEN** every one of them is answered with what was heard

### Requirement: A client of the copies starts without waiting for their discovery
Creating a client of a service's copies SHALL return at once; the copies SHALL be probed in the background, and the client's first request SHALL wait for the first probe to finish, so it never takes a copy that answers for one that does not.

#### Scenario: A client is created while the copies are slow to probe
- **GIVEN** probing the copies of a service takes a second
- **WHEN** a client of those copies is created
- **THEN** it returns at once, and its first request goes to a copy that answers
