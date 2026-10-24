## MODIFIED Requirements

### Requirement: One settings window opens from the menu
Choosing Settings in the menu SHALL show the settings window with its tabs (General, Dictation, Sounds, Paste, Archive, Logging, Services) and bring it to the front; choosing Settings again while the window exists SHALL bring the same window to the front instead of opening a second one. The window SHALL have no Refinement tab.

#### Scenario: Settings opens the window
- **GIVEN** the app runs
- **WHEN** the user chooses Settings in the menu
- **THEN** the settings window is in front with the General tab shown

#### Scenario: Settings chosen again reuses the window
- **GIVEN** the settings window is open behind another window
- **WHEN** the user chooses Settings in the menu again
- **THEN** the same settings window comes to the front
- **AND** no second settings window exists

#### Scenario: The window has no Refinement tab
- **GIVEN** the app runs
- **WHEN** the user chooses Settings in the menu
- **THEN** the tabs are General, Dictation, Sounds, Paste, Archive, Logging and Services
- **AND** no control names text refinement

## ADDED Requirements

### Requirement: Saving applies in the running app
Save SHALL write the edited values to the configuration file and apply the saved configuration to the running app, with any command line overrides of this run applied again, in the same process; the window SHALL say when the new settings are applied.

#### Scenario: A new hotkey and sound apply after Save
- **GIVEN** the app runs with the first hotkey and the first start sound
- **WHEN** the user picks a second hotkey and a second start sound and saves
- **THEN** the configuration file holds the second hotkey and the second start sound
- **AND** the window says the settings are applied
- **AND** the next dictation starts with the second hotkey and plays the second start sound
- **AND** the app was not relaunched

#### Scenario: A setting the running app cannot use is reported
- **GIVEN** the app runs
- **WHEN** the user saves a service address on this Mac where no service answers
- **THEN** the configuration file holds the new address
- **AND** the window says the settings were not applied and names the service
- **AND** the app keeps dictating with its previous settings

#### Scenario: A missing configuration file is created on Save
- **GIVEN** no configuration file exists
- **WHEN** the user changes the hotkey and saves
- **THEN** a configuration file exists that sets that hotkey
- **AND** every other option keeps its default value

### Requirement: Save keeps the user's configuration file as written
Save SHALL change only the options the user edited and SHALL keep every comment, the order of sections and options, and the format of every option the user did not edit; the file SHALL be replaced in one step, so it is never left half written.

#### Scenario: Comments and order survive a Save
- **GIVEN** a configuration file with comments above and beside its options
- **WHEN** the user changes one option and saves
- **THEN** the file differs from the original only in that option's value
- **AND** every comment is where it was

#### Scenario: Retired text refinement settings survive a Save
- **GIVEN** a configuration file that still holds the retired text refinement settings
- **WHEN** the user changes the logging level and saves
- **THEN** the retired text refinement settings in the file are unchanged

### Requirement: Command line overrides are noted in the window
An option that the command line of this run overrides SHALL show the value from the configuration file with a note naming the value the command line set for this run; Save SHALL still write the edited file value, and the override SHALL stay in force for this run.

#### Scenario: A hotkey overridden on the command line
- **GIVEN** the app was started with another hotkey on the command line than the file sets
- **WHEN** the user opens the General tab
- **THEN** the hotkey control shows the file's hotkey
- **AND** a note under it names the hotkey the command line set for this run

## REMOVED Requirements

### Requirement: Saving applies without a relaunch

**Reason**: Text refinement is removed; the requirement is restated without it as "Saving applies in the running app".
**Migration**: None.

### Requirement: The user's configuration file is preserved

**Reason**: Text refinement is removed; the requirement is restated without it as "Save keeps the user's configuration file as written".
**Migration**: None.

### Requirement: Command line overrides are shown

**Reason**: Text refinement is removed; the requirement is restated without it as "Command line overrides are noted in the window".
**Migration**: None.
