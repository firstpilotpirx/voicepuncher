## MODIFIED Requirements

### Requirement: An owned service that stops is started again
A service the app started SHALL be started again when its process exits, after 1 second, the pause doubling with every exit in a row up to 30 seconds and falling back to 1 second after a run of 30 seconds or more; a service or copy that once answered its health check and then fails three checks of it in a row SHALL be killed and started again the same way. Every service the app starts is health-checked, each copy of a speech recognition service on its own.

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

#### Scenario: An owned speech recognition copy that stops answering is restarted
- **GIVEN** the app started the speech recognition service for dictation and its copies are ready
- **WHEN** one copy stops answering without exiting
- **THEN** within 30 seconds the app has killed that copy and started it again, and it answers on its port
- **AND** the other copies were never restarted

## ADDED Requirements

### Requirement: The app runs every service as copies
When the app manages its services, it SHALL start the text refinement and command spotting services as the configured number of copies, 1 to 4, 2 by default, placed and found from the service's address as the copies of a speech recognition service are. A service already answering on its own port or its copy ports SHALL be adopted as it runs.

#### Scenario: Refinement on starts two copies of the text refinement service
- **GIVEN** no local service runs, refinement is on and the number of its copies is not configured
- **WHEN** the app launches
- **THEN** the text refinement service is started as two copies on its copy ports, and nothing is started on its own port

#### Scenario: Voice control starts two copies of the command spotting service
- **GIVEN** no local service runs, voice control is on and the number of its copies is not configured
- **WHEN** the app launches
- **THEN** the command spotting service is started as two copies on its copy ports

#### Scenario: One configured copy runs on the service's own port
- **GIVEN** refinement is on and its service is configured as one copy
- **WHEN** the app launches
- **THEN** the text refinement service is started once, on its own port

#### Scenario: A text refinement service the scripts run is adopted
- **GIVEN** the service scripts run the text refinement service on its own port
- **WHEN** the app launches with refinement on
- **THEN** the app adopts it and starts no copy of it
