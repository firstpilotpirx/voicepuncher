## MODIFIED Requirements

### Requirement: The service scripts start the services named
The service scripts SHALL start only the services the user names, and all of them when none is named.

#### Scenario: Only the speech recognition service is started
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the user starts only the speech recognition service for dictation with the service scripts
- **THEN** the speech recognition service for dictation answers and nothing answers on the ports of the speech recognition service for commands and the command spotting service

#### Scenario: The scripts know no text refinement service
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the user starts every service with the service scripts
- **THEN** the speech recognition services for dictation and for commands and the command spotting service answer
- **AND** no text refinement service is started and nothing answers on its former port 8080

### Requirement: A service that stops answering is started again
The service scripts SHALL check every few seconds that each service they keep running still answers — the command spotting service on its connection — and SHALL stop and start again a service that fails several checks in a row, writing the restart and its reason to that service's log. How often the scripts check SHALL be settable when they start, so a test proves the restart and the busy case through several checks without waiting the default cadence.

#### Scenario: The command spotting service hangs
- **GIVEN** the service scripts started the command spotting service
- **WHEN** its process stops answering without exiting
- **THEN** the service answers on its port again within 30 seconds, and its log notes that it was restarted because it stopped answering

#### Scenario: A busy service is not restarted
- **GIVEN** the service scripts started the command spotting service
- **WHEN** an app streams speech to it through several of the scripts' checks while the speech recognition service for commands answers its confirmations slowly
- **THEN** the service is not restarted

### Requirement: The app starts the services it needs
When the app runs from the hotkey and manages its services, it SHALL start at launch every local service its configuration needs that does not already answer on its port, with the arguments the service scripts use, SHALL count a service ready once it answers within 60 seconds of its start, and SHALL start dictation only once every needed service is ready. Transcribing a file SHALL start no service. The app SHALL never start, adopt or watch a text refinement service.

#### Scenario: A cold launch starts the needed services
- **GIVEN** no local service runs and voice control is off
- **WHEN** the app launches
- **THEN** the speech recognition service for dictation is started as its copies
- **AND** the speech recognition service for commands and the command spotting service are not started
- **AND** no text refinement service is started
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
- **GIVEN** the speech recognition service for dictation already answers
- **WHEN** the app launches
- **THEN** the app starts no service and dictation starts
- **AND** the log says the service was adopted
- **AND** after the app quits the service still answers

### Requirement: An owned service that stops is started again
A service the app started SHALL be started again when its process exits, after 1 second, the pause doubling with every exit in a row up to 30 seconds and falling back to 1 second after a run of 30 seconds or more; a service or copy that once answered its health check and then fails three checks of it in a row SHALL be killed and started again the same way. Every service the app starts is health-checked, each copy of a speech recognition service on its own.

#### Scenario: An owned service exits
- **GIVEN** the app started the command spotting service and it is ready
- **WHEN** its process exits
- **THEN** the app starts it again after 1 second and it is ready again

#### Scenario: The pause grows while a service keeps exiting
- **GIVEN** the app started a service that answers and then exits within seconds of every start
- **WHEN** it exited three times in a row
- **THEN** the app waited 1, 2 and 4 seconds before the starts that followed

#### Scenario: An owned service that stops answering is restarted
- **GIVEN** the app started the command spotting service and it is ready
- **WHEN** it fails three checks in a row without exiting
- **THEN** the app kills it and starts it again

#### Scenario: An owned speech recognition copy that stops answering is restarted
- **GIVEN** the app started the speech recognition service for dictation and its copies are ready
- **WHEN** one copy stops answering without exiting
- **THEN** within 30 seconds the app has killed that copy and started it again, and it answers on its port
- **AND** the other copies were never restarted

### Requirement: An owned service that cannot start is marked Failed
After five starts in a row of one service that each ended before the service answered — its process exited or 60 seconds passed — the app SHALL mark that service Failed with the reason and its log file, stop trying, and start it again only when the user asks to retry it.

#### Scenario: A service fails five starts
- **GIVEN** the command spotting service exits at every start
- **WHEN** the app started it five times
- **THEN** the service is Failed, naming its log file, and the app does not start it again
- **AND** dictation does not start

#### Scenario: Retry after a failure
- **GIVEN** the command spotting service is Failed
- **WHEN** the user asks to restart it and it now starts
- **THEN** the service is ready and dictation starts

### Requirement: Quit stops every owned service
When the app quits or is told to end — Quit in its menu, a quit from the system (logout, shutdown, Activity Monitor's Quit, any quit Apple Event), a termination or an interrupt signal — it SHALL log the quit, naming a quit from the system or the signal it got, and SHALL stop every service it started — the command spotting service first, then the speech recognition copies — by asking it to end and killing it after 5 seconds, and SHALL leave every adopted service running.

#### Scenario: Quit with owned and adopted services
- **GIVEN** the app started the speech recognition service for dictation and adopted the speech recognition service for commands
- **WHEN** the user quits the app
- **THEN** no copy of the speech recognition service for dictation is left
- **AND** the speech recognition service for commands still answers

#### Scenario: A service that ignores the request to end is killed
- **GIVEN** the app started a service that does not end when asked
- **WHEN** the user quits the app
- **THEN** the service is killed 5 seconds after it was asked to end

#### Scenario: A quit from the system stops the owned services
- **GIVEN** the app started the speech recognition service for dictation and it is ready
- **WHEN** the system tells the app to quit
- **THEN** the app's log names the quit from the system
- **AND** no process of the speech recognition service for dictation is left after the app ended

### Requirement: An adopted service that goes away is reported and waited for
When an adopted service stops answering three checks in a row, the app SHALL report it as stopped outside the app, SHALL NOT start its own on that port, and SHALL count it adopted again once it answers again.

#### Scenario: The service scripts stop an adopted service
- **GIVEN** the app adopted the speech recognition service for dictation
- **WHEN** the service stops answering
- **THEN** the service is shown as stopped outside the app and the app starts no speech recognition service for dictation
- **AND** once the service answers again it is shown as adopted

### Requirement: Each started service logs to its own file
The app SHALL write the output and errors of every service it starts to a file of its own in the logs folder, named after the service's program and port as the service scripts name it, rotated by size like the app's log, and SHALL log each start, readiness, exit, restart, adoption and failure of a service in its own log.

#### Scenario: A started service writes its own log
- **GIVEN** the app started the command spotting service on its port
- **WHEN** the service writes to its output
- **THEN** the text is in the log file named after the command spotting program and that port in the logs folder
- **AND** the app's log says when the service became ready

### Requirement: The engine in use is logged
At every launch that manages services, the app SHALL log for each service the engine it runs, where the engine came from — the bundle, a program the environment names, `$PATH` or the build folder — and the engine's version when it is known: the version file of a bundled engine, or the version folder of a Homebrew installation.

#### Scenario: A bundled engine is logged with its version
- **GIVEN** the app runs from a bundle whose speech recognition engine folder has the version file 1.9.5
- **WHEN** the app launches and looks up its services
- **THEN** its log says the speech recognition service runs whisper-server 1.9.5 from the bundle

#### Scenario: A Homebrew engine is logged with its version
- **GIVEN** the app runs outside a bundle and `$PATH` leads to whisper-server in Homebrew's folder of version 1.9.5
- **WHEN** the app launches and looks up its services
- **THEN** its log says the speech recognition service runs whisper-server 1.9.5 from `$PATH`

### Requirement: The app and the service scripts start a service alike
The app SHALL start each local service with the same arguments as the service scripts, and SHALL pick a model the same way the service scripts do: of several matching model files or folders, the newest. A difference between the two SHALL fail the tests.

#### Scenario: The app and the scripts start the speech recognition service with the same arguments
- **GIVEN** the service scripts and the app's own list of service command lines
- **WHEN** the arguments of the speech recognition services and the command spotting service are compared
- **THEN** each service gets the same arguments in the same order from both

#### Scenario: A changed argument in only one place is caught
- **GIVEN** a service argument that the service scripts set and the app does not
- **WHEN** the tests run
- **THEN** they fail, naming the service whose arguments differ

#### Scenario: Of two keyword spotting models the newest is used
- **GIVEN** two keyword spotting model folders in the models folder, the older one first by name
- **WHEN** the services the configuration needs are looked up
- **THEN** the command spotting service is started with the newer folder

## ADDED Requirements

### Requirement: A missing service program or model is named
A service whose program, or a model file named by its variable, cannot be found SHALL be Failed at once, without a start, with a reason naming what was looked for and where. A model of the catalog missing from the models folder does not reach the services: the runtime waits for models first.

#### Scenario: The command model is not on disk
- **GIVEN** the variable for the speech recognition model for commands names a file that is not on disk
- **WHEN** the services the configuration needs are looked up
- **THEN** the speech recognition service for commands is Failed with a reason naming the model file it looked for

### Requirement: The app runs the command spotting service as copies
When the app manages its services, it SHALL start the command spotting service as the configured number of copies, 1 to 4, 2 by default, found from the service's address as the copies of a speech recognition service are, on copy ports apart from those of every other service. A service already answering on its own port or its copy ports SHALL be adopted as it runs.

#### Scenario: Voice control starts two copies of the command spotting service
- **GIVEN** no local service runs, voice control is on and the number of its copies is not configured
- **WHEN** the app launches
- **THEN** the command spotting service is started as two copies on its copy ports, and nothing is started on its own port

#### Scenario: One configured copy runs on the service's own port
- **GIVEN** voice control is on and the command spotting service is configured as one copy
- **WHEN** the app launches
- **THEN** the command spotting service is started once, on its own port

#### Scenario: The copy ports of the services never overlap
- **GIVEN** every service on its default address
- **WHEN** the app works out the copy ports of each
- **THEN** no port is a copy port of two services

#### Scenario: A command spotting service the scripts run is adopted
- **GIVEN** the service scripts run the command spotting service on its own port
- **WHEN** the app launches with voice control on
- **THEN** the app adopts it and starts no copy of it

## REMOVED Requirements

### Requirement: A missing program or model is named

**Reason**: Text refinement is removed; the requirement is restated without it as "A missing service program or model is named".
**Migration**: None.

### Requirement: The app runs every service as copies

**Reason**: Text refinement is removed; the requirement is restated without it as "The app runs the command spotting service as copies".
**Migration**: None.
