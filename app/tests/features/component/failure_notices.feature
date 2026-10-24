# The pipeline runs on its own thread with stand-ins: the recorder hands over one second of
# audio (and, for a long dictation, a part during the recording and the last part at the stop),
# speech recognition and the paster answer or fail as the scenario sets them,
# and the overlay records what it was asked to show. The steps block while they watch the
# pipeline, so @serial keeps other scenarios from stalling them.
@failure_notices @component @serial
Feature: failure-notices

  Rule: A failed action shows a failure notice

    Scenario: The notice names the failing service and its remedy
      Given speech recognition is not running
      And the user is recording a hotkey dictation
      When the user stops the hotkey dictation
      Then within 2 seconds of the stop the overlay shows a failure notice naming speech recognition
      And the notice says to start the local services

  Rule: A lost dictation is announced

    Scenario: A hotkey dictation is lost
      Given speech recognition does not answer
      And the user is recording a hotkey dictation
      When the user stops the hotkey dictation
      Then the dictation's text is not pasted
      And the overlay shows a failure notice that the dictation was lost because speech recognition failed

    Scenario: A voice dictation is lost
      Given speech recognition does not answer
      And voice control is turned on and a dictation started by voice runs
      When the user ends the voice dictation by saying "stop dictation"
      Then the dictation's text is not pasted
      And the overlay shows a failure notice that the dictation was lost because speech recognition failed

  Rule: A partly lost dictation is announced

    Scenario: One chunk of a long dictation is lost
      Given speech recognition fails on one part of a long dictation and answers the others
      And the user is recording a long hotkey dictation
      When the user stops the hotkey dictation
      Then the text of the other parts is pasted
      And the overlay shows a failure notice that part of the dictation was lost

  Rule: A failed paste is announced

    Scenario: Pasting is refused
      Given pasting fails
      And the user is recording a hotkey dictation
      When the user stops the hotkey dictation
      Then the overlay shows a failure notice that the text was not pasted

  Rule: A microphone that does not start is announced

    Scenario: The microphone cannot start
      Given the microphone cannot be opened
      When the user presses the hotkey to record
      Then the recording does not start
      And the overlay shows a failure notice that the microphone is unavailable

  Rule: A microphone lost during a dictation is announced

    Scenario: The microphone fails during a hotkey dictation
      Given the user is recording a hotkey dictation
      When the input device fails
      Then the overlay shows a failure notice that the microphone failed
      And the dictation ends at once

    Scenario: The microphone fails during a voice dictation
      Given voice control is turned on and a dictation started by voice runs
      When the input device fails
      Then the overlay shows a failure notice that the microphone failed
      And the dictation ends at once

  Rule: A failure notice goes away on its own

    Scenario: A new dictation replaces the notice
      Given a failure notice is shown
      When the user presses the hotkey to record
      Then the recording starts and the overlay shows the recording instead of the notice

  Rule: A repeated failure does not stack notices

    Scenario: The same failure twice in a row
      Given a failure notice that speech recognition failed is shown
      When the next dictation fails the same way
      Then the overlay shows the same failure notice again without hiding in between

  Rule: A voice command that fails is announced

    Scenario: A key press is refused
      Given voice control is turned on and pressing keys in the focused app is refused
      When the user gives the voice command "press enter"
      Then the overlay shows a failure notice that "press enter" failed for lack of the Accessibility permission
      And the next voice command is still carried out

  Rule: Voice control that cannot start listening is announced

    Scenario: The command spotting service is not running at start
      Given voice control is turned on and the command spotting service is not running
      When the app is launched
      Then the overlay shows a failure notice that voice commands are unavailable because command spotting is not running
