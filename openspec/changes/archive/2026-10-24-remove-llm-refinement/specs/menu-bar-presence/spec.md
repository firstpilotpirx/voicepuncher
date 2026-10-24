## MODIFIED Requirements

### Requirement: The icon mirrors the dictation
While the runtime is ready, the icon SHALL mirror the stage of the dictation as the overlay shows it: recording — a filled microphone with a red dot; recognizing speech — a waveform; waiting for a text field — a keyboard; pasted — the ready icon again; a failure notice — a warning triangle for as long as the notice shows, then the ready icon. A command sign SHALL leave the icon as it is.

#### Scenario: A recording shows the recording icon
- **GIVEN** the runtime is ready
- **WHEN** a recording starts
- **THEN** the status item shows the recording icon

#### Scenario: The icon follows every stage of a dictation
- **GIVEN** a button has keyboard focus
- **WHEN** the user records a dictation with speech in it, ends it and then clicks into a text field
- **THEN** the status item shows the recording icon, then the waveform, then the keyboard, then the ready icon after the paste

#### Scenario: A failure notice shows the warning icon until it hides
- **GIVEN** the speech recognition service is not running
- **WHEN** the user stops a dictation
- **THEN** the status item shows the warning icon while the failure notice shows and the ready icon once it hides

#### Scenario: A command sign leaves the icon as it was
- **GIVEN** voice control is turned on and a recording runs
- **WHEN** the user says "clear text"
- **THEN** the status item keeps showing the recording icon

### Requirement: The menu lists the services
When the app manages its services, the menu SHALL offer a Services submenu that lists every service with its state in words — Ready, Starting, Restarting (with the try), Failed, Adopted, Stopped outside the app — and, for each, Restart, offered only for a service the app started, and Open Log, which shows the service's log file.

#### Scenario: The services are listed with their states
- **GIVEN** the app started the speech recognition service for dictation and adopted the one for commands
- **WHEN** the user opens the Services submenu
- **THEN** it lists the dictation copies as Ready and the speech recognition service for commands as Adopted
- **AND** Restart is offered for the dictation copies only

#### Scenario: Restart from the menu
- **GIVEN** the app started the command spotting service and it is Failed
- **WHEN** the user chooses Restart for it
- **THEN** the app starts it again
