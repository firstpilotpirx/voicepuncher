## ADDED Requirements

### Requirement: The app starts the services it needs
When the app runs from the hotkey and manages its services, it SHALL start at launch every local service its configuration needs that does not already answer on its port, with the arguments the service scripts use, SHALL count a service ready once it answers within 60 seconds of its start, and SHALL start dictation only once every needed service is ready. Transcribing a file SHALL start no service.

#### Scenario: A cold launch starts the needed services
- **GIVEN** no local service runs and voice control is off
- **WHEN** the app launches
- **THEN** the speech recognition service for dictation is started as its copies and the text refinement service is started, unless refinement is off
- **AND** the speech recognition service for commands and the command spotting service are not started
- **AND** dictation starts once every started service answers

#### Scenario: Voice control needs the command services
- **GIVEN** no local service runs and voice control is on
- **WHEN** the app launches
- **THEN** the speech recognition service for commands is started as its copies and the command spotting service is started besides the others

#### Scenario: A real launch leaves no service behind at quit
- **GIVEN** no local service runs on the chosen ports and the models are on disk
- **WHEN** the app launches, its services become ready and the app is told to end
- **THEN** each started service answered on its port while the app ran
- **AND** no process of them is left after the app ended

### Requirement: A service that already answers is adopted
A needed service whose port already answers as that service SHALL be adopted: the app SHALL NOT start it, SHALL count it ready, SHALL keep watching it, and SHALL leave it running when the app quits.

#### Scenario: Services kept warm by the service scripts are adopted
- **GIVEN** the speech recognition service for dictation and the text refinement service already answer
- **WHEN** the app launches
- **THEN** the app starts no service and dictation starts
- **AND** the log says each service was adopted
- **AND** after the app quits both services still answer

### Requirement: An owned service that stops is started again
A service the app started SHALL be started again when its process exits, after 1 second, the pause doubling with every exit in a row up to 30 seconds and falling back to 1 second after a run of 30 seconds or more; a service that once answered and then fails three checks of it in a row SHALL be killed and started again the same way.

#### Scenario: An owned service exits
- **GIVEN** the app started the text refinement service and it is ready
- **WHEN** its process exits
- **THEN** the app starts it again after 1 second and it is ready again

#### Scenario: The pause grows while a service keeps exiting
- **GIVEN** the app started a service that answers and then exits within seconds of every start
- **WHEN** it exited three times in a row
- **THEN** the app waited 1, 2 and 4 seconds before the starts that followed

#### Scenario: An owned service that stops answering is restarted
- **GIVEN** the app started the text refinement service and it is ready
- **WHEN** it fails three checks in a row without exiting
- **THEN** the app kills it and starts it again

### Requirement: An owned service that cannot start is marked Failed
After five starts in a row of one service that each ended before the service answered — its process exited or 60 seconds passed — the app SHALL mark that service Failed with the reason and its log file, stop trying, and start it again only when the user asks to retry it.

#### Scenario: A service fails five starts
- **GIVEN** the text refinement service exits at every start
- **WHEN** the app started it five times
- **THEN** the service is Failed, naming its log file, and the app does not start it again
- **AND** dictation does not start

#### Scenario: Retry after a failure
- **GIVEN** the text refinement service is Failed
- **WHEN** the user asks to restart it and it now starts
- **THEN** the service is ready and dictation starts

### Requirement: Quit stops every owned service
When the app quits or is told to end, it SHALL stop every service it started — the command spotting service first, then the speech recognition copies, then the text refinement service — by asking it to end and killing it after 5 seconds, and SHALL leave every adopted service running.

#### Scenario: Quit with owned and adopted services
- **GIVEN** the app started the speech recognition service for dictation and adopted the text refinement service
- **WHEN** the user quits the app
- **THEN** no copy of the speech recognition service for dictation is left
- **AND** the text refinement service still answers

#### Scenario: A service that ignores the request to end is killed
- **GIVEN** the app started a service that does not end when asked
- **WHEN** the user quits the app
- **THEN** the service is killed 5 seconds after it was asked to end

### Requirement: An adopted service that goes away is reported and waited for
When an adopted service stops answering three checks in a row, the app SHALL report it as stopped outside the app, SHALL NOT start its own on that port, and SHALL count it adopted again once it answers again.

#### Scenario: The service scripts stop an adopted service
- **GIVEN** the app adopted the text refinement service
- **WHEN** the service stops answering
- **THEN** the service is shown as stopped outside the app and the app starts no text refinement service
- **AND** once the service answers again it is shown as adopted

### Requirement: Each started service logs to its own file
The app SHALL write the output and errors of every service it starts to a file of its own in the logs folder, named after the service's program and port as the service scripts name it, rotated by size like the app's log, and SHALL log each start, readiness, exit, restart, adoption and failure of a service in its own log.

#### Scenario: A started service writes its own log
- **GIVEN** the app started the text refinement service on its port
- **WHEN** the service writes to its output
- **THEN** the text is in the log file named after the text refinement program and that port in the logs folder
- **AND** the app's log says when the service became ready

### Requirement: The services can be left to the service scripts
When the configuration says the app does not manage its services, the app SHALL start, adopt, watch and stop no service, and SHALL connect to them as it does without this requirement.

#### Scenario: The app leaves the services alone
- **GIVEN** the configuration says the app does not manage its services
- **WHEN** the app launches
- **THEN** the app starts no service and connects to the services that answer

### Requirement: A missing program or model is named
A service whose program or model file cannot be found SHALL be Failed at once, without a start, with a reason naming what was looked for and where.

#### Scenario: The refinement model is not on disk
- **GIVEN** the text refinement model file is not on disk
- **WHEN** the app launches
- **THEN** the text refinement service is Failed with a reason naming the model file it looked for
