# app-runtime Specification

## Purpose
The life of the application between launch and quit: the dictation pipeline starts from the configuration, stops, and restarts with new settings while the process keeps running, so the user never relaunches to apply a setting and user interface parts always know whether dictation is available.

## Requirements

### Requirement: The pipeline starts from the configuration
The runtime SHALL start the dictation pipeline from a valid configuration once every service it needs answers, and SHALL report the state Ready with whether voice control is on; when a service does not answer, it SHALL report the state Failed naming that service and start no pipeline.

#### Scenario: Every service answers
- **GIVEN** a valid configuration with voice control off
- **AND** the speech recognition service and the text refinement service answer
- **WHEN** the runtime starts
- **THEN** the dictation pipeline runs with that configuration
- **AND** the runtime state is Ready with voice control off

#### Scenario: A service does not answer
- **GIVEN** a valid configuration
- **AND** the speech recognition service does not answer
- **WHEN** the runtime starts
- **THEN** the runtime state is Failed with a reason naming the speech recognition service
- **AND** no dictation pipeline runs

### Requirement: A new configuration applies without a relaunch
The runtime SHALL apply a new valid configuration by restarting the pipeline inside the same process: after the reload the new hotkey starts a recording, the old hotkey starts none, and the pipeline uses the new settings.

#### Scenario: The hotkey changes on reload
- **GIVEN** the runtime is Ready with the first hotkey
- **WHEN** the runtime reloads a valid configuration with a second hotkey
- **THEN** the runtime state is Ready again
- **AND** a press of the second hotkey starts a recording
- **AND** a press of the first hotkey starts no recording
- **AND** the process is the same one

#### Scenario: Two reloads in a row
- **GIVEN** the runtime is Ready
- **WHEN** the runtime reloads two valid configurations one right after the other
- **THEN** the pipeline runs with the second configuration
- **AND** the runtime state is Ready

### Requirement: A restart waits for the dictation in progress
When a reload arrives while the pipeline records, recognizes, refines, waits for focus or pastes, the runtime SHALL report the state Restarting, let that dictation finish and paste, and only then restart the pipeline with the new configuration.

#### Scenario: Reload during a recording
- **GIVEN** the runtime is Ready
- **AND** a recording is in progress
- **WHEN** the runtime reloads a valid configuration
- **THEN** the runtime state is Restarting
- **AND** the recording keeps running until the hotkey stops it
- **AND** the dictation is pasted
- **AND** only then the pipeline restarts and the runtime state is Ready

#### Scenario: Reload while the pipeline waits for focus
- **GIVEN** the runtime is Ready
- **AND** a dictation waits for a text input to gain focus
- **WHEN** the runtime reloads a valid configuration
- **THEN** the runtime state is Restarting
- **AND** the pipeline restarts only after that dictation was pasted or given up

### Requirement: A rejected configuration keeps the running pipeline
When a reload arrives with a configuration that does not load or does not validate, the runtime SHALL keep the running pipeline with its settings, keep the state Ready, and log the reason.

#### Scenario: Reload with an invalid configuration
- **GIVEN** the runtime is Ready with the first hotkey
- **WHEN** the runtime reloads a configuration that does not validate
- **THEN** the runtime state stays Ready
- **AND** a press of the first hotkey still starts a recording
- **AND** the log names the reason the configuration was rejected

### Requirement: The runtime state is observable
The runtime SHALL tell every registered observer of every state change (Starting, Ready, Restarting, Stopped, Failed) in the order the changes happen, and SHALL tell a newly registered observer the current state at registration.

#### Scenario: An observer sees every state in order
- **GIVEN** an observer registered before the runtime starts
- **WHEN** the runtime starts, reloads a valid configuration and stops
- **THEN** the observer saw Starting, Ready, Restarting, Ready, Stopped in that order

#### Scenario: A late observer learns the current state
- **GIVEN** the runtime is Ready
- **WHEN** an observer registers
- **THEN** the observer is told Ready at once

### Requirement: A hang-up signal reloads the configuration
When the process receives the hang-up signal, the runtime SHALL load the configuration file it was started with, apply the command line overrides again, and reload with it as with any new configuration; a configuration that does not load or validate is rejected as any rejected configuration.

#### Scenario: Hang-up signal with an edited configuration file
- **GIVEN** the runtime is Ready from a configuration file
- **AND** the file was edited to a valid configuration with another hotkey
- **WHEN** the process receives the hang-up signal
- **THEN** the runtime reloads and the state is Ready
- **AND** the pipeline uses the hotkey from the edited file

#### Scenario: Hang-up signal with a broken configuration file
- **GIVEN** the runtime is Ready from a configuration file
- **AND** the file was edited so that it no longer validates
- **WHEN** the process receives the hang-up signal
- **THEN** the runtime state stays Ready
- **AND** the pipeline keeps its settings
- **AND** the log names the reason the file was rejected

### Requirement: Stopping the runtime ends the pipeline
When the runtime stops, it SHALL end the dictation pipeline without waiting for a dictation: a recording in progress is cancelled, voice control listening stops and the microphone closes, and the state becomes Stopped; the hotkey listener is the only part that may outlive the stop until the process ends.

#### Scenario: Stop while idle
- **GIVEN** the runtime is Ready with voice control on
- **WHEN** the runtime stops
- **THEN** the pipeline ends
- **AND** voice control listening stops
- **AND** the runtime state is Stopped

#### Scenario: Stop during a recording
- **GIVEN** the runtime is Ready
- **AND** a recording is in progress
- **WHEN** the runtime stops
- **THEN** the recording is cancelled and nothing is pasted
- **AND** the runtime state is Stopped

### Requirement: The hotkey rebinds without a relaunch
The hotkey listener SHALL match every key event against the current hotkey binding, so a binding changed by a reload takes effect at the next key event, with no new listener; a chord half pressed when the binding changes SHALL be forgotten.

#### Scenario: Single key rebinds
- **GIVEN** the binding is the first hotkey
- **WHEN** the binding changes to a second hotkey and the second hotkey is pressed and released
- **THEN** a press and a release are signalled
- **AND** a press of the first hotkey after the change signals nothing

#### Scenario: Chord half pressed when the binding changes
- **GIVEN** the binding is a chord and its modifier is held
- **WHEN** the binding changes to a single key and the chord's key is pressed
- **THEN** nothing is signalled

### Requirement: The runtime waits for permissions
Before it starts the pipeline, the runtime SHALL ask the permissions gate. With a permission missing it SHALL report the state Waiting for permissions, naming the missing permissions, start no pipeline, and leave that state for Starting once every permission is granted. A permission revoked while Ready SHALL stop the pipeline as a stop does and bring the state back to Waiting for permissions. Observers SHALL be told this state like every other.

#### Scenario: Start with a permission missing
- **GIVEN** an observer registered before the runtime starts
- **AND** Input Monitoring is not granted
- **WHEN** the runtime starts
- **THEN** the observer is told Waiting for permissions naming Input Monitoring
- **AND** no dictation pipeline runs

#### Scenario: The gate opens while waiting
- **GIVEN** the runtime is Waiting for permissions
- **WHEN** the last missing permission is granted
- **THEN** the observer is told Starting and then Ready
- **AND** the dictation pipeline runs

#### Scenario: A revocation while Ready
- **GIVEN** the runtime is Ready
- **WHEN** Accessibility is revoked
- **THEN** the pipeline ends
- **AND** the observer is told Waiting for permissions naming Accessibility

### Requirement: The runtime waits for the services it manages
When the app manages its services, the runtime SHALL start them at launch once every needed model file is present, also while it waits for permissions, SHALL report the state Starting services with how many needed services are ready out of how many, and SHALL start the pipeline only once every needed service is ready and every permission is granted.

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

### Requirement: A pipeline that fails stops in order and starts again
The runtime SHALL keep the process running when the dictation pipeline ends with an error or a panic: the pipeline's recording stops, the overlay hides, the failure is logged and a new pipeline starts from the same configuration; a new pipeline that fails within 30 seconds of that start SHALL leave the runtime Failed with the reason and no pipeline.

#### Scenario: The pipeline panics once
- **GIVEN** the runtime is Ready
- **WHEN** the dictation pipeline panics during a recording
- **THEN** the failure is logged and the overlay is hidden
- **AND** the runtime starts a new pipeline and is Ready again, and the hotkey starts a recording

#### Scenario: The restarted pipeline panics again at once
- **GIVEN** every pipeline the runtime starts panics on its first recording
- **WHEN** the user starts a recording twice
- **THEN** the runtime state is Failed with a reason naming the panic
- **AND** the process still runs
