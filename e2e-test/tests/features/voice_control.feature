# The phrases are spoken by macOS `say` and streamed to the real command spotting service as
# the app streams the microphone while listening; the app's recognizer decides what counts
# as a command. The hands-free message runs the real pipeline against the real service, with
# stand-ins for the recorder, speech recognition, paste and key presses; sounds and the hotkey
# are covered by the component scenarios and the manual run.
@voice_control @e2e
Feature: voice-control

  Rule: A command counts only when spoken on its own

    @smoke
    Scenario Outline: A command phrase spoken on its own is recognized
      Given the command spotting service is running
      When the user says "<phrase>" with a pause before and after it
      Then the "<phrase>" command takes effect

      Examples:
        | phrase           |
        | start dictation  |
        | stop dictation   |
        | cancel dictation |
        | press enter      |
        | clear all text   |

    @smoke
    Scenario Outline: A command said after a long time of listening is recognized
      Given the command spotting service is running
      And listening has been on for a long time with nothing said
      When the user says "<phrase>" with a pause before and after it
      Then the "<phrase>" command takes effect

      Examples:
        | phrase           |
        | start dictation  |
        | stop dictation   |
        | cancel dictation |
        | press enter      |
        | clear all text   |

    @smoke
    Scenario: A command phrase inside a sentence is ignored while listening
      Given the command spotting service is running
      When someone says "I will press enter when I am done" without pausing
      Then no command takes effect

    @accuracy
    Scenario: Ordinary speech triggers no command
      Given the command spotting service is running
      And a fixed sample of 20 recordings of the benchmark corpus, half Russian and half English, with reference transcripts
      When someone speaks every recording of the sample while listening
      Then no command takes effect

  Rule: The user's own way of saying the commands is recognized

    @smoke
    Scenario: A short Russian phrase is no command
      Given the command spotting service is running
      When someone says short Russian phrases, each on its own with a pause around it
      Then no command takes effect

  Rule: A command may end with "please"

    @smoke
    Scenario Outline: A command followed by please is recognized
      Given the command spotting service is running
      When the user says "<said>" with a pause before and after it
      Then the "<command>" command takes effect

      Examples:
        | said                    | command          |
        | start dictation please  | start dictation  |
        | stop dictation please   | stop dictation   |
        | cancel dictation please | cancel dictation |
        | press enter please      | press enter      |
        | clear all text please   | clear all text   |

  Rule: "clear text" erases the focused text too

    @smoke
    Scenario Outline: Clear text is recognized as clear all text
      Given the command spotting service is running
      When the user says "<said>" with a pause before and after it
      Then the "clear all text" command takes effect

      Examples:
        | said              |
        | clear text        |
        | clear text please |

  Rule: Commands are carried out in the order spoken

    @smoke
    Scenario: A message is dictated and sent by voice alone
      Given the command spotting service is running
      And listening is on
      When the user says "start dictation", a sentence, "stop dictation" and "press enter", each command on its own
      Then the sentence is pasted into the focused app
      And the Return key is pressed after it

  Rule: A whole hands-free session runs as said

    @smoke
    Scenario: A hands-free session in a synthesized voice
      Given the command spotting service is running
      And listening is on for a whole session
      When a synthesized voice goes through the hands-free session
      Then the app does everything the session said, in order

    @smoke @voice_samples
    Scenario: A hands-free session with the user's recorded commands
      Given the command spotting service is running
      And the user's own recordings of the commands exist on the machine
      And listening is on for a whole session
      When the hands-free session is spoken with the user's recorded commands
      Then the app does everything the session said, in order

    @smoke @voice_samples
    Scenario: A hands-free session with long pauses between its steps
      Given the command spotting service is running
      And the user's own recordings of the commands exist on the machine
      And listening is on for a whole session
      When the hands-free session is spoken with the user's recorded commands and long pauses
      Then the app does everything the session said, in order

  Rule: Voice control needs the command spotting service

    @smoke
    Scenario: The app refuses to start without the command spotting service
      Given voice control is turned on
      And the command spotting service is not running
      When the app starts
      Then the app exits with an error that names the command spotting service and how to start it
