## ADDED Requirements

### Requirement: The runtime waits for models
When the app manages its services, the runtime SHALL check the model files the configuration needs after the permissions and before it starts any service. With a file missing it SHALL report the state Waiting for models, naming the models folder and the needed files, start no service and no pipeline, and SHALL leave that state, start the services and go on as Starting services once every needed file is present.

#### Scenario: Start with a model missing
- **GIVEN** an observer registered before the runtime starts
- **AND** every permission is granted and the dictation model is missing
- **WHEN** the runtime starts
- **THEN** the observer is told Waiting for models
- **AND** no service is started and no dictation pipeline runs

#### Scenario: The last model arrives while waiting
- **GIVEN** the runtime is Waiting for models
- **WHEN** the missing file is placed in the models folder
- **THEN** the services are started
- **AND** the observer is told Starting services and then Ready

### Requirement: A reload to missing models waits for them
A reload whose configuration needs a model file that is missing SHALL end the running pipeline and bring the state back to Waiting for models.

#### Scenario: A reload to an empty models folder
- **GIVEN** the runtime is Ready
- **WHEN** the runtime reloads a configuration whose models folder is empty
- **THEN** the pipeline ends
- **AND** the observer is told Waiting for models

## MODIFIED Requirements

### Requirement: The runtime waits for the services it manages
When the app manages its services, the runtime SHALL start them at launch once every needed model file is present, also while it waits for permissions, SHALL report the state Starting services with how many needed services are ready out of how many, and SHALL start the pipeline only once every needed service is ready and every permission is granted.

#### Scenario: The pipeline starts once the services are ready
- **GIVEN** an observer registered before the runtime starts
- **AND** every permission is granted and two needed services are not ready yet
- **WHEN** the runtime starts and the services become ready one after the other
- **THEN** the observer is told Starting services with 0 of 2, then 1 of 2, then Ready
- **AND** the dictation pipeline runs
