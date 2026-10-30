## ADDED Requirements

### Requirement: The command recognizer is chosen in the configuration
The configuration SHALL name the command recognizer: the streaming recognizer (the default) or the keyword spotter confirmed by Whisper. With the streaming recognizer the app SHALL start the command spotting service with the streaming models and no speech recognition service for commands; with the keyword spotter, both as before. The service scripts SHALL use the keyword spotter unless told otherwise.

#### Scenario: Voice control starts the streaming recognizer by default
- **GIVEN** no local service runs, voice control is on and the user never chose a command recognizer
- **WHEN** the app launches
- **THEN** the command spotting service is started with the streaming models
- **AND** the speech recognition service for commands is not started

#### Scenario: The keyword spotter can still be chosen
- **GIVEN** no local service runs, voice control is on and the keyword spotter is chosen as the command recognizer
- **WHEN** the app launches
- **THEN** the speech recognition service for commands and the command spotting service with the keyword spotting model are started

## MODIFIED Requirements

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
- **GIVEN** no local service runs, voice control is on and the keyword spotter is the command recognizer
- **WHEN** the app launches
- **THEN** the speech recognition service for commands is started as its copies and the command spotting service is started besides the others

#### Scenario: A real launch leaves no service behind at quit
- **GIVEN** no local service runs on the chosen ports and the models are on disk
- **WHEN** the app launches, its services become ready and the app is told to end
- **THEN** each started service answered on its port while the app ran
- **AND** no process of them is left after the app ended

### Requirement: The app and the service scripts start a service alike
The app SHALL start each local service with the same arguments as the service scripts, and SHALL pick a model the same way the service scripts do: of several matching model files or folders, the newest. A difference between the two SHALL fail the tests.

#### Scenario: The app and the scripts start the speech recognition service with the same arguments
- **GIVEN** the service scripts and the app's own list of service command lines
- **WHEN** the arguments of the speech recognition services and the command spotting service are compared
- **THEN** each service gets the same arguments in the same order from both

#### Scenario: The app and the scripts start the streaming recognizer with the same arguments
- **GIVEN** the service scripts told to use the streaming recognizer and the app's own list of service command lines
- **WHEN** the arguments of the command spotting service with the streaming recognizer are compared
- **THEN** it gets the same arguments in the same order from both

#### Scenario: A changed argument in only one place is caught
- **GIVEN** a service argument that the service scripts set and the app does not
- **WHEN** the tests run
- **THEN** they fail, naming the service whose arguments differ

#### Scenario: Of two keyword spotting models the newest is used
- **GIVEN** two keyword spotting model folders in the models folder, the older one first by name
- **WHEN** the services the configuration needs are looked up
- **THEN** the command spotting service is started with the newer folder
