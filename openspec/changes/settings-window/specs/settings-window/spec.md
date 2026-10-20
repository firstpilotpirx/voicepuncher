# Spec Delta

## Purpose

A settings window lets the user see and change every option of the app without a terminal or a text editor: one tab per area, each option as a standard control, checked before it is saved, written to the configuration file with the user's own comments kept, and applied without a relaunch.

## ADDED Requirements

### Requirement: One settings window opens from the menu
Choosing Settings in the menu SHALL show the settings window with its tabs (General, Dictation, Refinement, Sounds, Paste, Archive, Logging) and bring it to the front; choosing Settings again while the window exists SHALL bring the same window to the front instead of opening a second one.

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
Every option the configuration file accepts SHALL appear in the settings window as a control on its tab, showing the value the file sets, or the default value when the file does not set it; service addresses and tuning options SHALL sit under an advanced disclosure on their tab.

#### Scenario: A value from the file is shown
- **GIVEN** the configuration file sets the hotkey to the right Command key
- **WHEN** the user opens the General tab
- **THEN** the hotkey control shows Right Command

#### Scenario: An option the file does not set shows its default
- **GIVEN** the configuration file does not set the minimum press length
- **WHEN** the user opens the General tab
- **THEN** the minimum press control shows the default length

#### Scenario: No option is left without a control
- **WHEN** the set of options the configuration file accepts is compared with the controls of the settings window
- **THEN** every option has exactly one control

#### Scenario: The microphone choice lists the Mac's input devices
- **GIVEN** the Mac has two input devices
- **WHEN** the user opens the microphone choice
- **THEN** it offers the system default and both devices by name

### Requirement: Saving applies without a relaunch
Save SHALL write the edited values to the configuration file and apply the saved configuration to the running app, with any command line overrides of this run applied again, in the same process; the window SHALL say when the new settings are applied.

#### Scenario: A new hotkey and sound apply after Save
- **GIVEN** the app runs with the first hotkey and the first start sound
- **WHEN** the user picks a second hotkey and a second start sound and saves
- **THEN** the configuration file holds the second hotkey and the second start sound
- **AND** the window says the settings are applied
- **AND** the next dictation starts with the second hotkey and plays the second start sound
- **AND** the app was not relaunched

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
