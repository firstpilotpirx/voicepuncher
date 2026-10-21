# settings-window Specification

## Purpose
A settings window lets the user see and change every option of the app without a terminal or a text editor: one tab per area, each option as a standard control, checked before it is saved, written to the configuration file with the user's own comments kept, and applied without a relaunch.

## Requirements

### Requirement: One settings window opens from the menu
Choosing Settings in the menu SHALL show the settings window with its tabs (General, Dictation, Refinement, Sounds, Paste, Archive, Logging, Services) and bring it to the front; choosing Settings again while the window exists SHALL bring the same window to the front instead of opening a second one.

#### Scenario: Settings opens the window
- **GIVEN** the app runs
- **WHEN** the user chooses Settings in the menu
- **THEN** the settings window is in front with the General tab shown

#### Scenario: Settings chosen again reuses the window
- **GIVEN** the settings window is open behind another window
- **WHEN** the user chooses Settings in the menu again
- **THEN** the same settings window comes to the front
- **AND** no second settings window exists

### Requirement: Every option has a control
Every option the configuration file accepts, except retired ones it only tolerates and the records the app keeps there itself (whether the login question was asked), SHALL appear in the settings window as a control on its tab, showing the value the file sets, or the default value when the file does not set it; service addresses and tuning options SHALL sit under an advanced disclosure on their tab.

#### Scenario: A value from the file is shown
- **GIVEN** the configuration file sets the hotkey to the right Command key
- **WHEN** the user opens the General tab
- **THEN** the hotkey control shows Right Command

#### Scenario: An option the file does not set shows its default
- **GIVEN** the configuration file does not set the minimum press length
- **WHEN** the user opens the General tab
- **THEN** the minimum press control shows the default length

#### Scenario: No option is left without a control
- **WHEN** the set of options the configuration file accepts, without the retired ones and the app's own records, is compared with the controls of the settings window
- **THEN** every option has exactly one control

#### Scenario: The app's own record has no control
- **WHEN** the controls of the settings window are listed
- **THEN** none of them edits whether the login question was asked

#### Scenario: The microphone choice lists the Mac's input devices
- **GIVEN** the Mac has two input devices
- **WHEN** the user opens the microphone choice
- **THEN** it offers the system default and both devices by name

### Requirement: The chosen microphone records
Dictation and voice control listening SHALL record from the input device the configuration names, and from the system default input device when it names the default; when the named device is not connected, they SHALL record from the system default device and log that the named one is missing.

#### Scenario: A named microphone records
- **GIVEN** the configuration names a connected input device that is not the system default
- **WHEN** the user dictates
- **THEN** the recording comes from the named device

#### Scenario: A named microphone that is not connected
- **GIVEN** the configuration names an input device that is not connected
- **WHEN** the user dictates
- **THEN** the recording comes from the system default device
- **AND** the log names the missing device

### Requirement: Saving applies without a relaunch
Save SHALL write the edited values to the configuration file and apply the saved configuration to the running app, with any command line overrides of this run applied again, in the same process; the window SHALL say when the new settings are applied.

#### Scenario: A new hotkey and sound apply after Save
- **GIVEN** the app runs with the first hotkey and the first start sound
- **WHEN** the user picks a second hotkey and a second start sound and saves
- **THEN** the configuration file holds the second hotkey and the second start sound
- **AND** the window says the settings are applied
- **AND** the next dictation starts with the second hotkey and plays the second start sound
- **AND** the app was not relaunched

#### Scenario: Refinement switched on applies after Save
- **GIVEN** the app runs with refinement off
- **WHEN** the user switches refinement on and saves
- **THEN** the next dictation is refined
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

### Requirement: Applying waits for the dictation in progress
When the user saves while a dictation is in progress, the window SHALL say that the settings apply after the current dictation, and SHALL say they are applied once that dictation was pasted and the app runs with the new settings.

#### Scenario: Save during a recording
- **GIVEN** a recording is in progress
- **WHEN** the user saves a changed setting
- **THEN** the window says the settings apply after the current dictation
- **AND** the recording continues until the user stops it
- **AND** once the dictation is pasted the window says the settings are applied

### Requirement: An invalid value is refused
Save SHALL check every value before writing; a value outside its allowed range, an address that is not on this machine, or a folder that does not exist and cannot be created SHALL be refused: nothing is written, nothing is applied, the field is marked and the window says why.

#### Scenario: A minimum press too short is refused
- **GIVEN** the configuration file holds a valid minimum press length
- **WHEN** the user enters a minimum press of 5 milliseconds and saves
- **THEN** the minimum press field is marked
- **AND** the window names the allowed range
- **AND** the configuration file is unchanged

#### Scenario: A service address on another machine is refused
- **WHEN** the user enters a speech recognition service address on another machine and saves
- **THEN** the address field is marked
- **AND** the window says the address must be on this machine
- **AND** the configuration file is unchanged

### Requirement: The user's configuration file is preserved
Save SHALL change only the options the user edited and SHALL keep every comment, the order of sections and options, and the format of every option the user did not edit; the file SHALL be replaced in one step, so it is never left half written.

#### Scenario: Comments and order survive a Save
- **GIVEN** a configuration file with comments above and beside its options
- **WHEN** the user changes one option and saves
- **THEN** the file differs from the original only in that option's value
- **AND** every comment is where it was

#### Scenario: The refinement examples survive a Save
- **GIVEN** a configuration file with refinement examples
- **WHEN** the user changes the logging level and saves
- **THEN** the refinement examples in the file are unchanged

### Requirement: Revert and unsaved edits
Revert SHALL show the values of the configuration file again, dropping unsaved edits; closing the window with unsaved edits SHALL ask whether to save them, drop them or keep the window open.

#### Scenario: Revert drops an unsaved edit
- **GIVEN** the user changed the hotkey without saving
- **WHEN** the user chooses Revert
- **THEN** the hotkey control shows the value from the file

#### Scenario: Closing with an unsaved edit asks first
- **GIVEN** the user changed the hotkey without saving
- **WHEN** the user closes the window
- **THEN** the window asks whether to save, not save or cancel
- **AND** choosing cancel keeps the window open with the edit

#### Scenario: Closing without edits asks nothing
- **GIVEN** the user changed nothing
- **WHEN** the user closes the window
- **THEN** the window closes without a question

### Requirement: Command line overrides are shown
An option that the command line of this run overrides SHALL show the value from the configuration file with a note naming the value the command line set for this run; Save SHALL still write the edited file value, and the override SHALL stay in force for this run.

#### Scenario: A hotkey overridden on the command line
- **GIVEN** the app was started with another hotkey on the command line than the file sets
- **WHEN** the user opens the General tab
- **THEN** the hotkey control shows the file's hotkey
- **AND** a note under it names the hotkey the command line set for this run

#### Scenario: Refinement switched off on the command line
- **GIVEN** the configuration turns refinement on
- **AND** the app was started with refinement turned off on the command line
- **WHEN** the user opens the Refinement tab
- **THEN** the refinement switch is on and a note says the command line turned it off for this run

### Requirement: Sounds can be heard before saving
Each sound choice SHALL have a play button that plays the chosen system sound at once, without saving and without a dictation; the choice None plays nothing.

#### Scenario: Previewing a sound
- **WHEN** the user picks a system sound for the stop sound and presses its play button
- **THEN** that sound plays
- **AND** the configuration file is unchanged

### Requirement: The Services tab holds the models and the services
The settings window SHALL have a Services tab, after Logging, holding the options of the `[services]` section and a Models row reading how many needed model files are present out of how many, with Download models… (which opens the Download window) and Show in Finder. Saving a new models folder SHALL apply as every other option, and the runtime SHALL check the models again in that folder.

#### Scenario: The Services tab shows the models
- **GIVEN** every needed model file is in the models folder
- **WHEN** the user opens the Services tab
- **THEN** it shows the models folder and reads that every model is present
- **AND** it offers Download models… and Show in Finder

#### Scenario: A new models folder is checked
- **GIVEN** the runtime is Ready
- **WHEN** the user picks an empty models folder in the Services tab and saves
- **THEN** the runtime waits for models and the Download window lists the files as Missing

### Requirement: A first launch creates the configuration file with the defaults
When the configuration file does not exist at launch, the app SHALL write it from a built-in default file before it reads its configuration, log that it created the file, and go on with those defaults; the default file SHALL set every option the settings window shows to its default value with a comment above it saying what the option means, and SHALL parse to the same configuration as an empty file. An existing configuration file SHALL never be replaced at launch.

#### Scenario: A first launch writes the default file
- **GIVEN** no configuration file exists
- **WHEN** the app starts
- **THEN** the configuration file exists with the default file's text
- **AND** the log says the file was created with the defaults

#### Scenario: The default file holds the defaults
- **WHEN** the default file is read as a configuration
- **THEN** it equals the configuration of an empty file

#### Scenario: The default file names every option
- **WHEN** the options of the default file are compared with the options the configuration accepts
- **THEN** every option the settings window shows is set in the default file, with a comment above it

#### Scenario: An existing file is kept
- **GIVEN** a configuration file the user edited
- **WHEN** the app starts
- **THEN** the file is unchanged
