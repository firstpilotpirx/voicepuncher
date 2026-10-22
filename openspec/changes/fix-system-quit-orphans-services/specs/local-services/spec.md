## MODIFIED Requirements

### Requirement: Quit stops every owned service
When the app quits or is told to end — Quit in its menu, a quit from the system (logout, shutdown, Activity Monitor's Quit, any quit Apple Event), a termination or an interrupt signal — it SHALL log which quit it got and SHALL stop every service it started — the command spotting service first, then the speech recognition copies, then the text refinement service — by asking it to end and killing it after 5 seconds, and SHALL leave every adopted service running.

#### Scenario: Quit with owned and adopted services
- **GIVEN** the app started the speech recognition service for dictation and adopted the text refinement service
- **WHEN** the user quits the app
- **THEN** no copy of the speech recognition service for dictation is left
- **AND** the text refinement service still answers

#### Scenario: A service that ignores the request to end is killed
- **GIVEN** the app started a service that does not end when asked
- **WHEN** the user quits the app
- **THEN** the service is killed 5 seconds after it was asked to end

#### Scenario: A quit from the system stops the owned services
- **GIVEN** the app started the speech recognition service for dictation and it is ready
- **WHEN** the system tells the app to quit
- **THEN** the app's log names the quit from the system
- **AND** no process of the speech recognition service for dictation is left after the app ended
