# The app's binary runs from its command line in a home of its own, with nothing on its
# standard input and the speech recognition service's address on a port where nothing
# listens. The rules that need the speech recognition service run in the E2E feature of the
# same name.
@file_transcription @component
Feature: file-transcription

  Rule: The command line is for developers and file mode

    Scenario: The help lists the developer options
      When the user asks the app for its help
      Then the help lists exactly the developer options
      And every option that changes a setting says it overrides the configuration for this run

    Scenario: A text refinement option is refused
      When the user starts the app with the former option that turned text refinement off
      Then the process ends with exit code 2 and names the option

  Rule: Exit codes are stable

    Scenario: An unknown option exits with 2
      When the user starts the app with an option it does not know
      Then the process ends with exit code 2 and names the option

    Scenario: File mode without its service exits with 2
      Given no speech recognition service answers at the configured address
      When the user transcribes the recording in file mode
      Then the process ends with exit code 2

    Scenario: File mode with an unreadable recording exits with 1
      Given a recording file that is not a WAV file
      When the user transcribes the recording in file mode
      Then the process ends with exit code 1
