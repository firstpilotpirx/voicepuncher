## ADDED Requirements

### Requirement: An adopted service that stops is replaced by the app's own
When an adopted service, or an adopted copy, stops answering three checks in a row and nothing listens on its port any more, the app SHALL log in one line the service, the copy, the port and the reason, SHALL start its own on that port as it starts a needed service, and SHALL from then on restart and stop it as a service it started.

#### Scenario: An adopted command spotting copy that stops is replaced
- **GIVEN** voice control is on and the app adopted both copies of the command spotting service
- **WHEN** the process of one copy ends
- **THEN** within 30 seconds the app has started its own copy on that port and it answers
- **AND** the log names the service, the copy, the port and that it stopped
- **AND** the menu shows the service ready again and voice commands work

#### Scenario: An adopted speech recognition copy that stops is replaced
- **GIVEN** the app adopted the copies of the speech recognition service for dictation
- **WHEN** the process of one copy ends
- **THEN** within 30 seconds the app has started its own copy on that port and it answers
- **AND** the other copies were never restarted

#### Scenario: A replaced copy is stopped at quit
- **GIVEN** the app replaced an adopted copy that stopped
- **WHEN** the user quits the app
- **THEN** the copy the app started is stopped and the copies still adopted keep answering

### Requirement: An adopted service that hangs is replaced by the app's own
When an adopted service, or an adopted copy, that answered its checks takes connections but fails three checks in a row, the app SHALL stop the program that listens on its port when that program is the service's own program, log it in one line naming the service, the copy, the port and the process id, and replace it as a stopped adopted service; another program on the port SHALL be left running and named in the log.

#### Scenario: A hung adopted copy is killed and replaced
- **GIVEN** the app adopted the copies of the speech recognition service for commands
- **WHEN** one copy stops answering without ending
- **THEN** within 30 seconds the app has stopped that copy's process and started its own copy on that port, and it answers
- **AND** the log names the service, the copy, the port and the process id it stopped

#### Scenario: Another program on the port is not killed
- **GIVEN** the app adopted a copy and then another program that is not that service takes its port and does not answer as the service
- **WHEN** the copy fails three checks in a row
- **THEN** the other program keeps running and the log names it with its process id

### Requirement: A service that answers again before its restart is adopted
Before each start or restart of a service or a copy, the app SHALL check its port; when the service already answers there (the service scripts restarted it meanwhile), the app SHALL adopt it instead of starting a second one, and a start that ends because its port was taken SHALL NOT count toward marking the service Failed.

#### Scenario: The service scripts restart a copy first
- **GIVEN** the app adopted a copy of the speech recognition service for dictation and the service scripts also watch it
- **WHEN** the copy ends and the scripts start it again before the app does
- **THEN** the app adopts the copy the scripts started and no second copy runs on that port
- **AND** the service is not Failed

## REMOVED Requirements

### Requirement: An adopted service that goes away is reported and waited for
**Reason**: An adopted service that stopped was waited for forever, so voice commands stayed dead until the user restarted the app (2026-10-27).
**Migration**: Replaced by "An adopted service that stops is replaced by the app's own" and "A service that answers again before its restart is adopted"; a service that comes back on its own is still adopted.
