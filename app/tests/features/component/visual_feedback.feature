# The pipeline runs on its own thread with the stand-ins of the text-insertion and
# voice-control scenarios: the focus is scripted, the command spotting service reports the
# phrases a scenario says, and the paster and overlay record what they were asked to do. The
# Pasted sign takes itself down after half a second, so a scenario checks that the overlay ends
# a pasted dictation on it and that nothing but a later signal replaces it. The steps block
# while they watch the pipeline, so @serial keeps other scenarios from stalling them.
@visual_feedback @component @serial
Feature: visual-feedback

  Rule: A pasted text shows the Pasted sign for half a second

    Scenario: A pasted dictation shows the Pasted sign
      Given a text field has keyboard focus
      When a dictation finishes
      Then the overlay shows the Pasted sign with no text window, then hides

    Scenario: Text pasted after a wait for focus shows the Pasted sign
      Given the app holds the text back because a button has focus
      When the user clicks into a text field
      Then the overlay shows the Pasted sign with no text window, then hides

    Scenario: Text left in the clipboard shows no Pasted sign
      Given the app holds the text back because a button has focus
      When the user presses the hotkey
      Then the overlay hides without showing the Pasted sign

    Scenario: A command right after the paste replaces the Pasted sign
      Given voice control is turned on and the overlay shows the Pasted sign
      When the user says "press enter"
      Then the enter sign replaces the Pasted sign at once
