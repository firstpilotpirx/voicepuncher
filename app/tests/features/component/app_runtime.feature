@app_runtime @component
Feature: app-runtime

  Rule: The pipeline starts from the configuration

    Scenario: Every service answers
      Given a valid configuration with voice control off
      And the speech recognition service and the text refinement service answer
      When the runtime starts
      Then the dictation pipeline runs with that configuration
      And the runtime state is Ready with voice control off

    Scenario: A service does not answer
      Given a valid configuration
      And the speech recognition service does not answer
      When the runtime starts
      Then the runtime state is Failed with a reason naming the speech recognition service
      And no dictation pipeline runs

  Rule: A new configuration applies without a relaunch

    Scenario: The hotkey changes on reload
      Given the runtime is Ready with the first hotkey
      When the runtime reloads a valid configuration with a second hotkey
      Then the runtime state is Ready again
      And a press of the second hotkey starts a recording
      And a press of the first hotkey starts no recording
      And the process is the same one

    Scenario: Two reloads in a row
      Given the runtime is Ready
      When the runtime reloads two valid configurations one right after the other
      Then the pipeline runs with the second configuration
      And the runtime state is Ready

  Rule: A restart waits for the dictation in progress

    Scenario: Reload during a recording
      Given the runtime is Ready
      And a recording is in progress
      When the runtime reloads a valid configuration
      Then the runtime state is Restarting
      And the recording keeps running until the hotkey stops it
      And the dictation is pasted
      And only then the pipeline restarts and the runtime state is Ready

    Scenario: Reload while the pipeline waits for focus
      Given the runtime is Ready
      And a dictation waits for a text input to gain focus
      When the runtime reloads a valid configuration
      Then the runtime state is Restarting
      And the pipeline restarts only after that dictation was pasted or given up

  Rule: A rejected configuration keeps the running pipeline

    Scenario: Reload with an invalid configuration
      Given the runtime is Ready with the first hotkey
      When the runtime reloads a configuration that does not validate
      Then the runtime state stays Ready
      And a press of the first hotkey still starts a recording
      And the log names the reason the configuration was rejected

  Rule: Stopping the runtime ends the pipeline

    Scenario: Stop while idle
      Given the runtime is Ready with voice control on
      When the runtime stops
      Then the pipeline ends
      And voice control listening stops
      And the runtime state is Stopped

    Scenario: Stop during a recording
      Given the runtime is Ready
      And a recording is in progress
      When the runtime stops
      Then the recording is cancelled and nothing is pasted
      And the runtime state is Stopped
