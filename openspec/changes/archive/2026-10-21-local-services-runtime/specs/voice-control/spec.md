## MODIFIED Requirements

### Requirement: Voice control needs the command spotting service to start
With voice control on and the services left to the service scripts, the system SHALL refuse to start, with a message naming the command spotting service and how to start it, when that service does not answer. When the app manages its services, it starts the command spotting service instead and waits for it.

#### Scenario: The app refuses to start without the command spotting service
- **GIVEN** voice control is turned on
- **AND** the app leaves its services to the service scripts
- **AND** the command spotting service is not running
- **WHEN** the app starts
- **THEN** the app exits with an error that names the command spotting service and how to start it
