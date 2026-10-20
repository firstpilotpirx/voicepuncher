# Spec Delta

## MODIFIED Requirements

### Requirement: A service that stops answering is started again
The service scripts SHALL check every few seconds that each service they keep running still answers — the command spotting service on its connection, the text refinement service on its health check — and SHALL stop and start again a service that fails several checks in a row, writing the restart and its reason to that service's log. How often the scripts check SHALL be settable when they start, so a test proves the restart and the busy case through several checks without waiting the default cadence.

#### Scenario: The command spotting service hangs
- **GIVEN** the service scripts started the command spotting service
- **WHEN** its process stops answering without exiting
- **THEN** the service answers on its port again within 30 seconds, and its log notes that it was restarted because it stopped answering

#### Scenario: A busy service is not restarted
- **GIVEN** the service scripts started the command spotting service
- **WHEN** an app streams speech to it through several of the scripts' checks while the speech recognition service for commands answers its confirmations slowly
- **THEN** the service is not restarted

### Requirement: The command spotting service answers while its confirmations are slow
The command spotting service SHALL keep taking in audio, spotting commands and answering on its connection while a confirmation by the speech recognition service for commands is slow, failed or hung; a confirmation that is not answered within a few seconds SHALL be given up and logged, never ending the connection. How long the service waits for a confirmation SHALL be settable when it starts, so a test proves the give-up without waiting the default.

#### Scenario: One confirmation hangs
- **GIVEN** an app streams speech to the command spotting service
- **AND** the speech recognition service for commands never answers one confirmation
- **WHEN** the app keeps streaming past the wait for that confirmation
- **THEN** the connection stays open and answers the app throughout, and the hung confirmation is logged as given up

#### Scenario: Commands are spotted after a hung confirmation
- **GIVEN** an app streams speech to the command spotting service
- **AND** the speech recognition service for commands never answered one confirmation
- **WHEN** the app streams the command "press enter" after it
- **THEN** the service reports "press enter"
