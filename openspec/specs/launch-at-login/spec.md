# launch-at-login Specification

## Purpose
TBD - created by archiving change launch-at-login. Update Purpose after archive.

## Requirements

### Requirement: The switch shows the system's state
The General tab of the settings window SHALL show a switch `Start voicepuncher when you log in` set from the system's login item status each time the tab is shown and each time the window comes to the front: on when the app is registered as a login item, off when it is not; a registration that waits for the user's approval SHALL show the switch on with a note saying it waits for approval in System Settings and a button that opens the Login Items settings.

#### Scenario: A registered app shows the switch on
- **GIVEN** the app is registered as a login item
- **WHEN** the user opens the General tab
- **THEN** the login switch is on
- **AND** no approval note is shown

#### Scenario: An unregistered app shows the switch off
- **GIVEN** the app is not registered as a login item
- **WHEN** the user opens the General tab
- **THEN** the login switch is off

#### Scenario: A registration waiting for approval
- **GIVEN** the app's login item waits for the user's approval in System Settings
- **WHEN** the user opens the General tab
- **THEN** the login switch is on
- **AND** a note says it waits for approval in System Settings
- **AND** a button opens the Login Items settings

#### Scenario: A change made in System Settings is shown
- **GIVEN** the General tab shows the login switch on
- **AND** the user removed the app from the login items in System Settings
- **WHEN** the settings window comes to the front again
- **THEN** the login switch is off

### Requirement: Switching on registers, switching off unregisters
Turning the login switch on SHALL register the app as a login item and turning it off SHALL unregister it, at once and without Save; the switch SHALL take no part in Save, Revert or the question about unsaved edits, and SHALL NOT be written to the configuration file.

#### Scenario: Switching on registers the app
- **GIVEN** the app is not registered as a login item
- **WHEN** the user turns the login switch on
- **THEN** the app is registered as a login item
- **AND** the configuration file is unchanged
- **AND** closing the window asks nothing

#### Scenario: Switching off unregisters the app
- **GIVEN** the app is registered as a login item
- **WHEN** the user turns the login switch off
- **THEN** the app is no longer registered as a login item

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

### Requirement: A failure is told
When registering or unregistering fails, the window SHALL show the system's reason in its status line and the login switch SHALL return to the position it had before.

#### Scenario: A refused registration
- **GIVEN** the app is not registered as a login item
- **AND** the system refuses to register it
- **WHEN** the user turns the login switch on
- **THEN** the status line shows the system's reason
- **AND** the login switch is off

### Requirement: The offer is made once
The first time the installed app reaches Ready (every permission granted, services ready) and the configuration file does not record that the question was asked, the app SHALL ask whether to start voicepuncher at login with the answers `Start at Login` and `Not Now`; `Start at Login` SHALL register the app as a login item; either answer SHALL be recorded in the configuration file so the question is never asked again; an app that is not installed, or that is already registered, SHALL NOT ask.

#### Scenario: The first Ready asks and Start at Login registers
- **GIVEN** the installed app has never asked about starting at login
- **WHEN** the app reaches Ready
- **THEN** the app asks whether to start at login
- **AND** answering Start at Login registers the app as a login item
- **AND** the configuration file records that the question was asked

#### Scenario: Not Now registers nothing and is remembered
- **GIVEN** the installed app has never asked about starting at login
- **WHEN** the app reaches Ready and the user answers Not Now
- **THEN** the app is not registered as a login item
- **AND** the configuration file records that the question was asked

#### Scenario: The question is not asked again
- **GIVEN** the configuration file records that the question was asked
- **WHEN** the app reaches Ready after a relaunch
- **THEN** the app asks nothing

#### Scenario: Ready again after a reload asks nothing
- **GIVEN** the installed app asked about starting at login in this run
- **WHEN** a saved setting restarts the pipeline and the app reaches Ready again
- **THEN** the app asks nothing more

#### Scenario: An app that is not installed asks nothing
- **GIVEN** the app runs from its build folder
- **WHEN** the app reaches Ready
- **THEN** the app asks nothing
- **AND** the configuration file is unchanged

### Requirement: A launch at login is logged
When the system starts the app at login, the app SHALL start as on any other launch and its log SHALL say that it was launched at login.

#### Scenario: Started by the login item
- **GIVEN** the app is registered as a login item
- **WHEN** the system starts the app at login
- **THEN** the log says the app was launched at login
