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
