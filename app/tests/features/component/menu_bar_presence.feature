# The real runtime runs pipelines built from stand-ins; the overlay they drive goes through the
# status item's decorator to a quiet overlay, and the runtime tells the status item's observer
# its states. A status item double records every icon it is asked to draw, and opening the
# menu reads the menu the status item would show at that moment. The steps block while they
# watch the runtime, so @serial keeps other scenarios from stalling them.
@menu_bar_presence @component @serial
Feature: menu-bar-presence

  Rule: The menu bar shows the app from launch

    Scenario: The status item appears while the app starts
      Given the app was just launched and its runtime is still starting
      When the menu bar is looked at
      Then the status item shows the waiting icon

    Scenario: The ready icon shows once the runtime is ready
      Given the status item shows the waiting icon while the runtime starts
      When the runtime becomes ready
      Then the status item shows the ready icon

    Scenario: A restart shows the waiting icon and ready again afterwards
      Given the runtime is ready and no dictation runs
      When the runtime restarts with a new configuration and becomes ready again
      Then the status item shows the waiting icon during the restart and the ready icon afterwards

    Scenario: A failed runtime shows the failed icon until the app exits
      Given the runtime is starting
      When the runtime fails because a local service does not answer
      Then the status item shows the failed icon and the start reports the failure the app exits on

  Rule: The icon mirrors the dictation

    Scenario: A recording shows the recording icon
      Given the runtime is ready and no dictation runs
      When the user presses the hotkey to start a recording
      Then the status item shows the recording icon

    Scenario: The icon follows every stage of a dictation
      Given text refinement is enabled and a button has keyboard focus while the runtime is ready
      When the user dictates a sentence, ends the dictation and then clicks into a text field
      Then the status item shows the recording icon, then the waveform, then the keyboard, then the ready icon after the paste

    Scenario: A failure notice shows the warning icon until it hides
      Given the speech recognition service is not running while a recording runs
      When the user stops the dictation with the hotkey
      Then the status item shows the warning icon while the failure notice shows and the ready icon once it hides

    Scenario: A command sign leaves the icon as it was
      Given voice control is turned on and a recording runs
      When the user clears the text by voice
      Then the status item keeps showing the recording icon

  Rule: The icon shows voice control

    Scenario: Voice control listening shows the listening icon
      Given voice control is turned on and no dictation runs yet
      When the runtime becomes ready
      Then the status item shows the listening icon

    Scenario: Commands unavailable shows the warning badge on the ear
      Given voice control is turned on, the runtime is ready and no dictation runs
      When the command spotting service has been away long enough for the commands-unavailable sign
      Then the status item shows the ear with a warning badge until the sign goes

    Scenario: A dictation outranks the commands-unavailable sign
      Given the commands-unavailable sign shows
      When the user presses the hotkey to start a recording
      Then the status item shows the recording icon

  Rule: The menu tells the state in words

    Scenario: The ready status line names the hotkey
      Given the runtime is ready with the hotkey "RightOption" in the recording mode "hold"
      When the user opens the menu
      Then the menu's first line names the app and its version
      And the status line reads "Ready — hold Right Option to dictate"

    Scenario: The ready status line in Toggle mode says press
      Given the runtime is ready with the hotkey "RightOption" in the recording mode "toggle"
      When the user opens the menu
      Then the status line reads "Ready — press Right Option to dictate"

    Scenario: The recording status line says how to stop
      Given a recording runs with the hotkey "RightOption" in the recording mode "hold"
      When the user opens the menu
      Then the status line reads "Recording — release Right Option to stop"

    Scenario: The failed status line gives the reason
      Given the runtime failed because the speech recognition service does not answer
      When the user opens the menu
      Then the status line starts with "Failed:" and names the speech recognition service

    Scenario: The status line reflects the state at the moment the menu opens
      Given the menu was opened and closed while the runtime was ready
      When the user opens the menu again during speech recognition
      Then the status line reads "Recognizing speech…"

    Scenario Outline: Every hotkey has a human name
      Given the runtime is ready with the hotkey "<key>" in the recording mode "toggle"
      When the user opens the menu
      Then the status line reads "Ready — press <name> to dictate"

      Examples:
        | key          | name          |
        | RightOption  | Right Option  |
        | RightControl | Right Control |
        | RightShift   | Right Shift   |
        | RightCommand | Right Command |
        | LeftOption   | Left Option   |
        | LeftControl  | Left Control  |
        | Fn           | Fn            |
        | CapsLock     | Caps Lock     |
        | Space        | Space         |
        | CmdE         | Cmd+E         |
        | LeftOptionE  | Option+E      |
        | F13          | F13           |
        | F19          | F19           |

  Rule: Quit ends the application

    Scenario: Quit while idle ends the process cleanly
      Given the runtime is ready and no dictation runs
      When the user chooses Quit in the menu
      Then the runtime stops and the process ends with exit code 0

    Scenario: Quit during a recording cancels it
      Given a recording runs with the hotkey "RightOption" in the recording mode "toggle"
      When the user chooses Quit in the menu
      Then the recording is cancelled with the cancel sound and nothing is pasted
      And the runtime stops and the process ends with exit code 0
