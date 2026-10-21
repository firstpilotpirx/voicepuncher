@launch_at_login @component
Feature: launch-at-login

  Rule: The switch shows the system's state

    Scenario: A registered app shows the switch on
      Given the app is registered as a login item
      When the user opens the General tab
      Then the login switch is on
      And no approval note is shown

    Scenario: An unregistered app shows the switch off
      Given the app is not registered as a login item
      When the user opens the General tab
      Then the login switch is off

    Scenario: A registration waiting for approval
      Given the app's login item waits for the user's approval
      When the user opens the General tab
      Then the login switch is on
      And a note says it waits for approval in System Settings
      And a button opens the Login Items settings

    Scenario: A change made in System Settings is shown
      Given the app is registered as a login item
      And the user opens the General tab
      And the user removed the app from the login items in System Settings
      When the settings window comes to the front again
      Then the login switch is off

  Rule: Switching on registers, switching off unregisters

    Scenario: Switching on registers the app
      Given the app is not registered as a login item
      And the user opens the General tab
      When the user turns the login switch on
      Then the app is registered as a login item
      And the configuration file is left as it was
      And closing the settings window asks nothing

    Scenario: Switching off unregisters the app
      Given the app is registered as a login item
      And the user opens the General tab
      When the user turns the login switch off
      Then the app is not registered as a login item

  Rule: Unavailable outside an installed app

    Scenario: Run from a build folder
      Given the app runs from its build folder
      When the user opens the General tab
      And the user turns the login switch on
      Then the login switch is disabled
      And a note says it is available when voicepuncher runs as an installed app
      And nothing was registered or unregistered

  Rule: A failure is told

    Scenario: A refused registration
      Given the app is not registered as a login item
      And the system refuses to register it
      And the user opens the General tab
      When the user turns the login switch on
      Then the status line shows the system's reason
      And the login switch is off

  Rule: The offer is made once

    Scenario: The first Ready asks and Start at Login registers
      Given the installed app has never asked about starting at login
      When the app reaches Ready
      Then the app asks whether to start at login
      And answering Start at Login registers the app as a login item
      And the configuration file records that the question was asked

    Scenario: Not Now registers nothing and is remembered
      Given the installed app has never asked about starting at login
      When the app reaches Ready
      And the user answers Not Now
      Then the app is not registered as a login item
      And the configuration file records that the question was asked

    Scenario: The question is not asked again
      Given the configuration file records that the question was asked
      When the app reaches Ready after a relaunch
      Then the app asks nothing

    Scenario: Ready again after a reload asks nothing
      Given the installed app has never asked about starting at login
      And the app reached Ready and asked whether to start at login
      When a saved setting restarts the pipeline and the app reaches Ready again
      Then the app asks nothing more

    Scenario: An app that is not installed asks nothing
      Given the app runs from its build folder
      When the app reaches Ready
      Then the app asks nothing
      And the configuration file is left as it was

  Rule: A launch at login is logged

    Scenario: Started by the login item
      Given the app is registered as a login item
      When the system starts the app at login
      Then the log says the app was launched at login
