## ADDED Requirements

### Requirement: The menu shows the services starting
While the runtime starts its services, the status item SHALL show the waiting icon and the menu's status line SHALL read "Starting services… (N of M ready)"; a runtime failed because a service cannot start SHALL show the failed icon and keep the app running so the user can retry it.

#### Scenario: Services still loading at launch
- **GIVEN** the runtime starts its services and 2 of 5 are ready
- **WHEN** the user opens the menu
- **THEN** the status item shows the waiting icon
- **AND** the status line reads "Starting services… (2 of 5 ready)"

### Requirement: The menu lists the services
When the app manages its services, the menu SHALL offer a Services submenu that lists every service with its state in words — Ready, Starting, Restarting (with the try), Failed, Adopted, Stopped outside the app — and, for each, Restart, offered only for a service the app started, and Open Log, which shows the service's log file.

#### Scenario: The services are listed with their states
- **GIVEN** the app started the speech recognition service for dictation and adopted the text refinement service
- **WHEN** the user opens the Services submenu
- **THEN** it lists the dictation copies as Ready and the text refinement service as Adopted
- **AND** Restart is offered for the dictation copies only

#### Scenario: Restart from the menu
- **GIVEN** the app started the text refinement service and it is Failed
- **WHEN** the user chooses Restart for it
- **THEN** the app starts it again
