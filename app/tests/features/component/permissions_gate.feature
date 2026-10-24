@permissions_gate @component
Feature: permissions-gate

  Rule: Dictation starts only with every permission granted

    Scenario: Launch with every permission granted
      Given Microphone, Accessibility and Input Monitoring are granted
      When the app launches in hotkey mode
      Then the dictation pipeline starts
      And the Permissions window does not open

    Scenario: Launch with a permission missing
      Given Accessibility is not granted and the other two are
      When the app launches in hotkey mode
      Then no dictation pipeline runs
      And the Permissions window opens with the Accessibility row marked not granted
      And the process keeps running

    Scenario: The last grant starts dictation
      Given the app runs without a pipeline because Input Monitoring is missing
      When the user grants Input Monitoring
      Then within 2 s the dictation pipeline starts in the same process

  Rule: Not asked yet differs from denied

    Scenario: A permission never requested
      Given the app never asked for Microphone
      When the Permissions window shows
      Then the Microphone row shows Not asked yet
      And the microphone is not opened to find out

    Scenario: A permission denied
      Given the user denied Microphone
      When the Permissions window shows
      Then the Microphone row shows Not granted

  Rule: A revoked permission stops dictation

    Scenario: Accessibility revoked while idle
      Given the pipeline runs and no dictation is in progress
      When the user removes Accessibility in System Settings
      Then within 5 s the pipeline stops
      And the Permissions window opens with the Accessibility row marked not granted and the line saying Accessibility was revoked

    Scenario: Microphone revoked during a recording
      Given a recording is in progress
      When the user removes Microphone in System Settings
      Then within 5 s the recording is cancelled with the cancel sound and nothing is pasted
      And a failure notice says the Microphone permission was revoked

    Scenario: Granting a revoked permission again resumes dictation
      Given the pipeline stopped because Accessibility was revoked
      When the user grants Accessibility again
      Then the dictation pipeline starts again without a relaunch

  Rule: Permission changes are logged

    Scenario: A change is logged
      Given the app runs with Microphone granted, Accessibility not granted and Input Monitoring not asked yet
      When the user grants Accessibility
      Then the log has one line naming Microphone granted, Accessibility granted and Input Monitoring not asked yet

  Rule: File mode needs no permission

    Scenario: File transcription without permissions
      Given no permission is granted
      When the user transcribes a recording file
      Then the transcript is printed
      And no permission is checked

  Rule: Each permission is shown with its status and its remedy

    Scenario: Open System Settings lands on the permission's page
      Given the user denied Microphone
      When the user presses Open System Settings in the Microphone row
      Then System Settings opens on the Microphone page of Privacy & Security

    Scenario: Request shows the system prompt
      Given the app never asked for Input Monitoring
      When the user presses Request… in the Input Monitoring row
      Then the system asks the user for Input Monitoring
      And the Input Monitoring row shows Checking… until the status is known

    Scenario: A granted row has no button
      Given Microphone is granted
      When the Permissions window shows
      Then the Microphone row shows Granted and no button

    Scenario: The Accessibility pane is named as macOS 27 names it
      Given the app runs on macOS 27 and Accessibility is not granted
      When the Permissions window shows the Accessibility row
      Then the row names the pane "Device Control and Data Access"

    Scenario: The Accessibility pane keeps its old name before macOS 27
      Given the app runs on macOS 26 and Accessibility is not granted
      When the Permissions window shows the Accessibility row
      Then the row names the pane "Accessibility"

  Rule: The window names who must be granted

    Scenario: Run from a terminal
      Given the app runs as a bare binary started from iTerm2
      When the Permissions window shows
      Then it says voicepuncher runs from iTerm2 and the permissions must be granted to iTerm2

    Scenario: Run from the bundle
      Given the app runs from its bundle
      When the Permissions window shows
      Then it names no other application to grant the permissions to

  Rule: The window is current and can be opened at any time

    Scenario: A grant in System Settings shows up in the open window
      Given the Permissions window is open with Accessibility not granted
      When the user grants Accessibility
      Then within 1 s the Accessibility row shows Granted

    Scenario: Continue waits for every permission
      Given the Permissions window is open with Accessibility not granted
      When the user looks at the footer
      Then Continue is disabled

    Scenario: Continue closes the window once all are granted
      Given all three permissions are granted and the window is open
      When the user presses Continue
      Then the window closes and the dictation pipeline is running

  Rule: A switch that does not take is explained

    Scenario: Accessibility switched on for an earlier build
      Given the user pressed Open System Settings in the Accessibility row while Accessibility is not granted
      When the Permissions window checks again
      Then the Accessibility row says to remove voicepuncher with − and add it again with +

    Scenario: No hint before System Settings was opened
      Given Accessibility is not granted and the other two are
      When the Permissions window shows
      Then the Accessibility row shows no hint

  Rule: Input Monitoring can be asked for again after a denial

    Scenario: Input Monitoring denied
      Given Input Monitoring is not granted and the app runs from its bundle
      When the user presses Open System Settings in the Input Monitoring row
      Then the system is asked for Input Monitoring again
      And System Settings opens on the Input Monitoring page
      And Finder shows the voicepuncher bundle
      And the Input Monitoring row says to add voicepuncher with + when it is not in the list
