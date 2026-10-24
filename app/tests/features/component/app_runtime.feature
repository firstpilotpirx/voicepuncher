@app_runtime @component
Feature: app-runtime

  Rule: The pipeline starts from the configuration

    Scenario: Every service answers
      Given a valid configuration with voice control off
      And the speech recognition service answers
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

  Rule: The runtime state is observable

    Scenario: An observer sees every state in order
      Given an observer registered before the runtime starts
      When the runtime starts, reloads a valid configuration and stops
      Then the observer saw Starting, Ready, Restarting, Ready, Stopped in that order

    Scenario: A late observer learns the current state
      Given the runtime is Ready
      When an observer registers
      Then the observer is told Ready at once

  # The process sends the signal to itself; @serial keeps one scenario's signal from
  # reaching another scenario's runtime.
  Rule: A hang-up signal reloads the configuration

    @serial
    Scenario: Hang-up signal with an edited configuration file
      Given the runtime is Ready from a configuration file
      And the file was edited to a valid configuration with another hotkey
      When the process receives the hang-up signal
      Then the runtime reloads and the state is Ready
      And the pipeline uses the hotkey from the edited file

    @serial
    Scenario: Hang-up signal with a broken configuration file
      Given the runtime is Ready from a configuration file
      And the file was edited so that it no longer validates
      When the process receives the hang-up signal
      Then the log names the reason the file was rejected
      And the runtime state stays Ready
      And the pipeline keeps its settings

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

  Rule: The hotkey rebinds without a relaunch

    Scenario: Single key rebinds
      Given the binding is the first hotkey
      When the binding changes to a second hotkey and the second hotkey is pressed and released
      Then a press and a release are signalled
      And a press of the first hotkey after the change signals nothing

    Scenario: Chord half pressed when the binding changes
      Given the binding is a chord and its modifier is held
      When the binding changes to a single key and the chord's key is pressed
      Then nothing is signalled

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

  Rule: The runtime waits for models

    Scenario: Start with a model missing
      Given an observer registered before the runtime starts
      And every permission is granted and the dictation model is missing
      When the runtime starts
      Then the observer is told Waiting for models
      And no service is started and no dictation pipeline runs

    Scenario: The last model arrives while waiting
      Given the runtime is Waiting for models
      When the missing file is placed in the models folder
      Then the services are started
      And the observer is told Starting services and then Ready once the models are present

  Rule: A reload to missing models waits for them

    Scenario: A reload to an empty models folder
      Given the runtime is Ready with every model present
      When the runtime reloads a configuration whose models folder is empty
      Then the pipeline ends and the observer is told Waiting for models

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

  Rule: A pipeline that fails stops in order and starts again

    Scenario: The pipeline panics once
      Given the runtime is Ready
      When the dictation pipeline panics during a recording
      Then the failure is logged and the overlay is hidden
      And the runtime starts a new pipeline and is Ready again, and the hotkey starts a recording

    Scenario: The restarted pipeline panics again at once
      Given every pipeline the runtime starts panics on its first recording
      When the user starts a recording twice
      Then the runtime state is Failed with a reason naming the panic
      And the process still runs
