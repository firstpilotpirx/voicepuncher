## MODIFIED Requirements

### Requirement: Unavailable outside an installed app
When the app does not run from an application bundle, the login switch SHALL be disabled, drawn dimmed so it does not look like a switch that responds, with a note saying it is available when voicepuncher runs as an installed app, and the app SHALL NOT ask the system to register or unregister anything. An app run from its bundle whose login item the system does not know yet — it was never registered — SHALL show the switch off and enabled.

#### Scenario: Run from a build folder
- **GIVEN** the app runs from its build folder, not as an installed app
- **WHEN** the user opens the General tab
- **THEN** the login switch is disabled and drawn dimmed
- **AND** a note says it is available when voicepuncher runs as an installed app
- **AND** nothing was registered or unregistered

#### Scenario: A bundle that never registered can switch
- **GIVEN** the app runs from its bundle
- **AND** its login item was never registered, so the system does not know it yet
- **WHEN** the user opens the General tab
- **THEN** the login switch is off and enabled
- **AND** no note is shown

#### Scenario: Switching on a bundle that never registered
- **GIVEN** the app runs from its bundle and its login item was never registered
- **WHEN** the user turns the login switch on
- **THEN** the system is asked to register the app as a login item
