## ADDED Requirements

### Requirement: The app and the service scripts start a service alike
The app SHALL start each local service with the same arguments as the service scripts, and SHALL pick a model the same way the service scripts do: of several matching model files or folders, the newest. A difference between the two SHALL fail the tests.

#### Scenario: The app and the scripts start the speech recognition service with the same arguments
- **GIVEN** the service scripts and the app's own list of service command lines
- **WHEN** the arguments of the speech recognition service, the text refinement service and the command spotting service are compared
- **THEN** each service gets the same arguments in the same order from both

#### Scenario: A changed argument in only one place is caught
- **GIVEN** a service argument that the service scripts set and the app does not
- **WHEN** the tests run
- **THEN** they fail, naming the service whose arguments differ

#### Scenario: Of two keyword spotting models the newest is used
- **GIVEN** two keyword spotting model folders in the models folder, the older one first by name
- **WHEN** the services the configuration needs are looked up
- **THEN** the command spotting service is started with the newer folder
