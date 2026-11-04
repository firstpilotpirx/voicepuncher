@change_workflow @e2e
Feature: change-workflow

  # The session's requests must not queue behind the other scenarios': @serial.
  @serial
  Rule: Voice control smoke scenarios not under timing stream faster than real time

    @regression
    Scenario: A hands-free session not under timing ends before its audio would
      Given the command spotting service is running
      And listening is on for a whole session
      When a synthesized voice goes through the hands-free session
      Then the app does everything the session said, in order
      And the session ended in less than a tenth of the time its audio lasts
