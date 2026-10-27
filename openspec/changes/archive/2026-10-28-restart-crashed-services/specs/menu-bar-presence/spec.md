## MODIFIED Requirements

### Requirement: The menu lists the services
When the app manages its services, the menu SHALL offer a Services submenu that lists every service with its state in words — Ready, Starting, Restarting (with the try), Failed, Adopted — and, for each, Restart, offered only for a service the app started, and Open Log, which shows the service's log file.

#### Scenario: The services are listed with their states
- **GIVEN** the app started the speech recognition service for dictation and adopted the one for commands
- **WHEN** the user opens the Services submenu
- **THEN** it lists the dictation copies as Ready and the speech recognition service for commands as Adopted
- **AND** Restart is offered for the dictation copies only

#### Scenario: Restart from the menu
- **GIVEN** the app started the command spotting service and it is Failed
- **WHEN** the user chooses Restart for it
- **THEN** the app starts it again

