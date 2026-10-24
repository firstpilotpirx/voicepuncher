## MODIFIED Requirements

### Requirement: Each started service logs to its own file
The app SHALL write the output and errors of every service it starts to a file of its own in the logs folder, named after the service's program and port as the service scripts name it, and SHALL log each start, readiness, exit, restart, adoption and failure of a service in its own log. The service SHALL write into that file itself, not through the app, so its log goes on when the app that started it quits and a later app adopts it. The file SHALL be rotated by size when a service starts, like the app's log.

#### Scenario: A started service writes its own log
- **GIVEN** the app started the command spotting service on its port
- **WHEN** the service writes to its output
- **THEN** the text is in the log file named after the command spotting program and that port in the logs folder
- **AND** the app's log says when the service became ready

#### Scenario: A service keeps its log after the app that started it quits
- **GIVEN** the app started a service
- **WHEN** the app quits without stopping it and the service writes to its output
- **THEN** the text is in the service's log file
