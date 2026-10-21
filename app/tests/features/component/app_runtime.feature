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

  Rule: The runtime waits for permissions

    Scenario: Start with a permission missing
      Given an observer registered before the runtime starts
      And Input Monitoring is not granted
      When the runtime starts
      Then the observer is told Waiting for permissions naming Input Monitoring
      And no dictation pipeline runs

    Scenario: The gate opens while waiting
      Given the runtime is Waiting for permissions
      When the last missing permission is granted
      Then the observer is told Starting and then Ready
      And the dictation pipeline runs

    Scenario: A revocation while Ready
      Given the runtime is Ready
      When Accessibility is revoked
      Then the pipeline ends
      And the observer is told Waiting for permissions naming Accessibility

  Rule: The runtime waits for the services it manages

    Scenario: The pipeline starts once the services are ready
      Given an observer registered before the runtime starts
      And every permission is granted and two needed services are not ready yet
      When the runtime starts and the services become ready one after the other
      Then the observer is told Starting services with 0 of 2, then 1 of 2, then Ready
      And the dictation pipeline runs

  Rule: A service that cannot start fails the runtime

    Scenario: A service that cannot start fails the runtime without ending it
      Given the runtime waits for its services
      When a needed service is marked Failed
      Then the runtime state is Failed naming that service
      And no dictation pipeline runs and the process keeps running

    Scenario: A retried service brings the runtime back
      Given the runtime is Failed because a service could not start
      When the service is retried and becomes ready
      Then the observer is told Starting services and then Ready

  Rule: A service lost while Ready keeps the pipeline

    Scenario: A service lost while Ready keeps the pipeline
      Given the runtime is Ready with services it started
      When one of them stops answering
      Then the runtime state stays Ready and the pipeline keeps running

  Rule: A reload starts the services it needs

    Scenario: Turning voice control on starts the command services
      Given the runtime is Ready with voice control off and services it started
      When the runtime reloads a configuration with voice control on
      Then the services voice control needs are started
      And the runtime becomes Ready with voice control on once they are ready

  Rule: Stopping the runtime stops its services

    Scenario: Stopping the runtime stops its services
      Given the runtime is Ready with services it started
      When the runtime stops
      Then the services it started are stopped
