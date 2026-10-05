# The pipeline runs on its own thread with stand-ins: the recorder records one second, the
# speech recognition service answers a fixed transcript, the focus is scripted, and the
# paster and overlay record what they were asked to do. The steps block while they watch the
# pipeline, so @serial keeps other scenarios from stalling them.
@text_insertion @slice @serial
Feature: text-insertion

  Rule: Text is pasted at once unless no text input is surely focused

    Scenario: Paste at once into a focused text field
      Given a text field has keyboard focus
      When a dictation finishes
      Then the text is pasted at once
      And the overlay never shows that it waits for focus

    Scenario: Paste at once when the app does not tell what has focus
      Given the focused app does not report its focused element
      When a dictation finishes
      Then the text is pasted at once

    Scenario: Paste at once into an element of a generic kind
      Given the focused element is web content of a browser or an Electron app
      When a dictation finishes
      Then the text is pasted at once

    Scenario: Paste at once into a list that offers a text selection
      Given the focused element is a list that offers a text selection
      When a dictation finishes
      Then the text is pasted at once

    Scenario: Hold the text back when a button has focus
      Given a button has keyboard focus
      When a dictation finishes
      Then nothing is pasted yet
      And the overlay shows that the app waits for focus

  Rule: Held-back text is pasted when a text input gets focus

    Scenario: Paste once the user clicks into a text field
      Given the app holds the text back because a button has focus
      When the user clicks into a text field
      Then the text is pasted into it
      And the overlay hides

  Rule: A wait that ends without a text input leaves the text in the clipboard

    Scenario: The wait time passes without a text input
      Given the app holds the text back because a button has focus
      When the wait time passes and the button still has focus
      Then the clipboard holds the text
      And nothing is pasted
      And the overlay hides

    Scenario: A hotkey press ends the wait
      Given the app holds the text back because a button has focus
      When the user presses the hotkey
      Then the clipboard holds the text
      And nothing is pasted
      And no recording starts

    Scenario: The next hotkey press after an ended wait records
      Given a hotkey press ended a wait for focus
      When the user presses the hotkey again
      Then a recording starts

  Rule: The wait time is a setting

    Scenario: A wait time of zero pastes at once
      Given the config file sets the wait time to 0
      And a button has keyboard focus
      When a dictation finishes
      Then the text is pasted at once
