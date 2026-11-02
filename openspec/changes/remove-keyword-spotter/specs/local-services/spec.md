## ADDED Requirements

### Requirement: The app and the service scripts start each service alike
The app SHALL start each local service with the same arguments as the service scripts, and SHALL pick a model the same way the service scripts do: of several matching model files or folders, the newest. A difference between the two SHALL fail the tests.

#### Scenario: The app and the scripts start the speech recognition service with the same arguments
- **GIVEN** the service scripts and the app's own list of service command lines
- **WHEN** the arguments of the speech recognition service for dictation and the command spotting service are compared
- **THEN** each service gets the same arguments in the same order from both

#### Scenario: The app and the scripts start the streaming recognizer with the same arguments
- **GIVEN** the service scripts and the app's own list of service command lines
- **WHEN** the arguments of the command spotting service are compared
- **THEN** it gets the same arguments in the same order from both

#### Scenario: A changed argument in only one place is caught
- **GIVEN** a service argument that the service scripts set and the app does not
- **WHEN** the tests run
- **THEN** they fail, naming the service whose arguments differ

### Requirement: A service whose program or model is missing is named
A service whose program, or a model file named by its variable, cannot be found SHALL be Failed at once, without a start, with a reason naming what was looked for and where. A model of the catalog missing from the models folder does not reach the services: the runtime waits for models first.

#### Scenario: The dictation model is not on disk
- **GIVEN** the variable for the speech recognition model for dictation names a file that is not on disk
- **WHEN** the services the configuration needs are looked up
- **THEN** the speech recognition service for dictation is Failed with a reason naming the model file it looked for

## MODIFIED Requirements

### Requirement: The service scripts start the services named
The service scripts SHALL start only the services the user names, and all of them when none is named.

#### Scenario: Only the speech recognition service is started
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the user starts only the speech recognition service for dictation with the service scripts
- **THEN** the speech recognition service for dictation answers and nothing answers on the port of the command spotting service

#### Scenario: The scripts know no text refinement service
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the user starts every service with the service scripts
- **THEN** the speech recognition service for dictation and the command spotting service answer
- **AND** no text refinement service is started and nothing answers on its former port 8080

### Requirement: A service that stops answering is started again
The service scripts SHALL check every few seconds that each service they keep running still answers — the command spotting service on its connection — and SHALL stop and start again a service that fails several checks in a row, writing the restart and its reason to that service's log. How often the scripts check SHALL be settable when they start, so a test proves the restart and the busy case through several checks without waiting the default cadence.

#### Scenario: The command spotting service hangs
- **GIVEN** the service scripts started the command spotting service
- **WHEN** its process stops answering without exiting
- **THEN** the service answers on its port again within 30 seconds, and its log notes that it was restarted because it stopped answering

#### Scenario: A busy service is not restarted
- **GIVEN** the service scripts started the command spotting service
- **WHEN** an app streams speech to it through several of the scripts' checks while every core of the machine is busy
- **THEN** the service is not restarted

### Requirement: The app starts the services it needs
When the app runs from the hotkey and manages its services, it SHALL start at launch every local service its configuration needs that does not already answer on its port, with the arguments the service scripts use, SHALL count a service ready once it answers within 60 seconds of its start, and SHALL start dictation only once every needed service is ready. Transcribing a file SHALL start no service. The app SHALL never start, adopt or watch a text refinement service.

#### Scenario: A cold launch starts the needed services
- **GIVEN** no local service runs and voice control is off
- **WHEN** the app launches
- **THEN** the speech recognition service for dictation is started as its copies
- **AND** the command spotting service is not started
- **AND** no text refinement service is started
- **AND** dictation starts once every started service answers

#### Scenario: Voice control needs the command services
- **GIVEN** no local service runs, voice control is on
- **WHEN** the app launches
- **THEN** the command spotting service is started as its copies besides the others
- **AND** no speech recognition service for commands is started

#### Scenario: A real launch leaves no service behind at quit
- **GIVEN** no local service runs on the chosen ports and the models are on disk
- **WHEN** the app launches, its services become ready and the app is told to end
- **THEN** each started service answered on its port while the app ran
- **AND** no process of them is left after the app ended

### Requirement: Quit stops every owned service
When the app quits or is told to end — Quit in its menu, a quit from the system (logout, shutdown, Activity Monitor's Quit, any quit Apple Event), a termination or an interrupt signal — it SHALL log the quit, naming a quit from the system or the signal it got, and SHALL stop every service it started — the command spotting service first, then the speech recognition copies — by asking it to end and killing it after 5 seconds, and SHALL leave every adopted service running.

#### Scenario: Quit with owned and adopted services
- **GIVEN** the app started the speech recognition service for dictation and adopted the command spotting service
- **WHEN** the user quits the app
- **THEN** no copy of the speech recognition service for dictation is left
- **AND** the command spotting service still answers

#### Scenario: A service that ignores the request to end is killed
- **GIVEN** the app started a service that does not end when asked
- **WHEN** the user quits the app
- **THEN** the service is killed 5 seconds after it was asked to end

#### Scenario: A quit from the system stops the owned services
- **GIVEN** the app started the speech recognition service for dictation and it is ready
- **WHEN** the system tells the app to quit
- **THEN** the app's log names the quit from the system
- **AND** no process of the speech recognition service for dictation is left after the app ended

### Requirement: A copy of a speech recognition service is ready once it has transcribed speech
The service scripts and the app SHALL count a copy of a speech recognition service they start as ready only after it answered the transcription of a short utterance, and SHALL report the copy ready together with how long that first transcription took, so the first request a client sends a copy never waits for the copy's first decode. A copy the app starts again after it stopped SHALL be warmed the same way before it counts ready; a copy the scripts' supervisor starts again after a crash is warmed by its first request instead. The checks of a running copy SHALL send it no speech.

#### Scenario: The service scripts report a copy ready after its first transcription
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the service scripts start the speech recognition service for dictation
- **THEN** each copy is reported ready with the time of its first transcription, and the copy's log shows that transcription before the report

#### Scenario: The app counts a started copy ready after its first transcription
- **GIVEN** the app started a copy of a speech recognition service
- **WHEN** the copy answers on its port but has not transcribed the utterance the app sent it yet
- **THEN** the app does not count the copy ready until the copy transcribed that utterance

#### Scenario: A copy that cannot transcribe in time is not ready
- **GIVEN** the app started a copy of a speech recognition service that answers on its port
- **WHEN** the copy does not answer the transcription of the utterance within the time a start may take
- **THEN** the copy counts as not ready in time and is started again, as a copy that never answered

#### Scenario: The check of a ready copy sends it no speech
- **GIVEN** a copy the app started is ready
- **WHEN** the app checks the copy while it runs
- **THEN** the check asks only whether the copy answers, and sends it no speech to transcribe

### Requirement: An adopted service that hangs is replaced by the app's own
When an adopted service, or an adopted copy, that answered its checks takes connections but fails three checks in a row, the app SHALL stop the program that listens on its port when that program is the service's own program, log it in one line naming the service, the copy, the port and the process id, and replace it as a stopped adopted service; another program on the port SHALL be left running and named in the log.

#### Scenario: A hung adopted copy is killed and replaced
- **GIVEN** the app adopted the copies of the speech recognition service for dictation
- **WHEN** one copy stops answering without ending
- **THEN** within 30 seconds the app has stopped that copy's process and started its own copy on that port, and it answers
- **AND** the log names the service, the copy, the port and the process id it stopped

#### Scenario: Another program on the port is not killed
- **GIVEN** the app adopted a copy and then another program that is not that service takes its port and does not answer as the service
- **WHEN** the copy fails three checks in a row
- **THEN** the other program keeps running and the log names it with its process id

## REMOVED Requirements

### Requirement: The command spotting service answers while its confirmations are slow
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: The command spotting service does not pile up confirmations
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: Speech that is no command is confirmed quickly
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: Voice commands are confirmed by a speech recognition service of their own
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: A hung confirmation makes the command spotting service unhealthy
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: Confirmations waiting in the command spotting service are bounded
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: The command recognizer is chosen in the configuration
**Reason**: the keyword spotter and its Whisper confirmation are removed; commands are heard only by the streaming recognizers.
**Migration**: none; the streaming recognizer reports each command once, without a confirmation.

### Requirement: The app and the service scripts start a service alike
**Reason**: its scenarios of the keyword spotter's confirmation are gone with it.
**Migration**: replaced by "The app and the service scripts start each service alike".

### Requirement: A missing service program or model is named
**Reason**: its scenarios of the keyword spotter's confirmation are gone with it.
**Migration**: replaced by "A service whose program or model is missing is named".
