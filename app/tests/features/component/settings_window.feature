@settings_window @component
Feature: settings-window

  Rule: Every option has a control

    Scenario: A value from the file is shown
      Given the configuration file sets the hotkey to the right Command key
      When the user opens the settings
      Then the hotkey control shows Right Command

    Scenario: An option the file does not set shows its default
      Given the configuration file does not set the minimum press length
      When the user opens the settings
      Then the minimum press control shows the default length

    Scenario: No option is left without a control
      When the options the configuration file accepts are compared with the settings controls
      Then every option has exactly one control

    Scenario: The app's own record has no control
      When the controls of the settings window are listed
      Then none of them edits whether the login question was asked

    Scenario: The microphone choice lists the Mac's input devices
      Given the Mac has two input devices
      When the user opens the settings
      Then the microphone choice offers the system default and both devices by name

  Rule: Saving applies in the running app

    Scenario: A new hotkey and sound apply after Save
      Given the app runs from its configuration file with the first hotkey and the first start sound
      And the user opens the settings
      When the user picks the second hotkey and the second start sound and saves
      Then the configuration file holds the second hotkey and the second start sound
      And the settings say the settings are applied
      And the next dictation starts with the second hotkey and plays the second start sound
      And the app was not relaunched

    Scenario: A setting the running app cannot use is reported
      Given the app runs from its configuration file with the first hotkey and the first start sound
      And the user opens the settings
      When the user saves a speech recognition address on this Mac where no service answers
      Then the configuration file holds the new address
      And the settings say the settings were not applied and name the speech recognition service
      And the app keeps dictating with the first hotkey

    Scenario: A missing configuration file is created on Save
      Given no configuration file exists
      And the user opens the settings
      When the user picks the second hotkey and saves
      Then a configuration file exists that sets the second hotkey
      And every other option of that file keeps its default value

  Rule: Applying waits for the dictation in progress

    Scenario: Save during a recording
      Given the app runs from its configuration file with the first hotkey and the first start sound
      And the user opens the settings
      And a recording is in progress
      When the user picks the second hotkey and saves
      Then the settings say the settings apply after the current dictation
      And the recording keeps running until the hotkey stops it
      And once the dictation is pasted the settings say the settings are applied

  Rule: An invalid value is refused

    Scenario: A minimum press too short is refused
      Given the configuration file holds a valid minimum press length
      And the user opens the settings
      When the user enters a minimum press of 5 milliseconds and saves
      Then the minimum press field is marked
      And the settings name the allowed range
      And the configuration file is unchanged

    Scenario: A service address on another machine is refused
      Given the configuration file holds a valid minimum press length
      And the user opens the settings
      When the user enters a speech recognition address on another machine and saves
      Then the speech recognition address field is marked
      And the settings say the address must be on this machine
      And the configuration file is unchanged

  Rule: Save keeps the user's configuration file as written

    Scenario: Comments and order survive a Save
      Given a configuration file with comments above and beside its options
      And the user opens the settings
      When the user changes the logging level and saves
      Then the file differs from the original only in the logging level
      And every comment is where it was

    Scenario: Retired text refinement settings survive a Save
      Given a configuration file that still holds the retired text refinement settings
      And the user opens the settings
      When the user changes the logging level and saves
      Then the retired text refinement settings in the file are unchanged

  Rule: Revert and unsaved edits

    Scenario: Revert drops an unsaved edit
      Given the configuration file sets the hotkey to the right Command key
      And the user opens the settings
      And the user changed the hotkey without saving
      When the user chooses Revert
      Then the hotkey control shows Right Command

    Scenario: Closing with an unsaved edit asks first
      Given the configuration file sets the hotkey to the right Command key
      And the user opens the settings
      And the user changed the hotkey without saving
      When the user closes the settings
      Then the settings ask whether to save, not save or cancel
      And choosing cancel keeps the settings open with the edit

    Scenario: Closing without edits asks nothing
      Given the configuration file sets the hotkey to the right Command key
      And the user opens the settings
      When the user closes the settings
      Then the settings close without a question

  Rule: Command line overrides are noted in the window

    Scenario: A hotkey overridden on the command line
      Given the configuration file sets the hotkey to the right Command key
      And the app was started with the hotkey F13 on the command line
      When the user opens the settings
      Then the hotkey control shows Right Command
      And a note under it names F13 as the hotkey the command line set for this run

  Rule: One settings window opens from the menu

    Scenario: The window has no Refinement tab
      When the user opens the settings
      Then the tabs are General, Dictation, Sounds, Paste, Archive, Logging and Services
      And no control names text refinement
