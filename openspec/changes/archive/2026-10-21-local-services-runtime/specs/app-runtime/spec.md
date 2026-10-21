## ADDED Requirements

### Requirement: The runtime waits for the services it manages
When the app manages its services, the runtime SHALL start them at launch, also while it waits for permissions, SHALL report the state Starting services with how many needed services are ready out of how many, and SHALL start the pipeline only once every needed service is ready and every permission is granted.

#### Scenario: The pipeline starts once the services are ready
- **GIVEN** an observer registered before the runtime starts
- **AND** every permission is granted and two needed services are not ready yet
- **WHEN** the runtime starts and the services become ready one after the other
- **THEN** the observer is told Starting services with 0 of 2, then 1 of 2, then Ready
- **AND** the dictation pipeline runs

### Requirement: A service that cannot start fails the runtime
A needed service marked Failed SHALL bring the runtime to the state Failed naming that service, without ending the process; a retry of that service SHALL bring the runtime back to Starting services.

#### Scenario: A service that cannot start fails the runtime without ending it
- **GIVEN** the runtime waits for its services
- **WHEN** a needed service is marked Failed
- **THEN** the runtime state is Failed naming that service
- **AND** no dictation pipeline runs and the process keeps running

#### Scenario: A retried service brings the runtime back
- **GIVEN** the runtime is Failed because a service could not start
- **WHEN** the service is retried and becomes ready
- **THEN** the observer is told Starting services and then Ready

### Requirement: A service lost while Ready keeps the pipeline
A service that stops answering after the pipeline started SHALL NOT stop the pipeline nor change the runtime state.

#### Scenario: A service lost while Ready keeps the pipeline
- **GIVEN** the runtime is Ready with services it started
- **WHEN** one of them stops answering
- **THEN** the runtime state stays Ready and the pipeline keeps running

### Requirement: A reload starts the services it needs
A reload whose configuration needs a service not yet started SHALL start it and restart the pipeline with that configuration once it is ready.

#### Scenario: Turning voice control on starts the command services
- **GIVEN** the runtime is Ready with voice control off
- **WHEN** the runtime reloads a configuration with voice control on
- **THEN** the services voice control needs are started
- **AND** the runtime becomes Ready with voice control on once they are ready

### Requirement: Stopping the runtime stops its services
Stopping the runtime SHALL stop the services it started, after the pipeline ended.

#### Scenario: Stopping the runtime stops its services
- **GIVEN** the runtime is Ready with services it started
- **WHEN** the runtime stops
- **THEN** the services it started are stopped
