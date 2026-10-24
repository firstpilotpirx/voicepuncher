# The pipeline runs on its own thread with stand-ins: the recorder records one second, the
# speech recognition service answers a fixed transcript, the focus is scripted, and the
# paster and overlay record what they were asked to do. The clipboard rule runs the macOS
# paster on a pasteboard of its own, with a paste keystroke that sends nothing. The steps
# block while they watch the pipeline, so @serial keeps other scenarios from stalling them.
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

    Scenario: Paste at once into a text input on a web page
      Given a text input on a web page has keyboard focus
      When a dictation finishes
      Then the text is pasted at once

    Scenario: Paste at once into an editable element on a web page
      Given an editable element on a web page has keyboard focus
      When a dictation finishes
      Then the text is pasted at once

    Scenario: Hold the text back when a web page has focus without a text input
      Given a web page has keyboard focus and no text input on it has
      When a dictation finishes
      Then nothing is pasted yet
      And the overlay shows that the app waits for focus

    Scenario: Hold the text back when text is selected on a web page
      Given a web page with selected text has keyboard focus and no text input on it has
      When a dictation finishes
      Then nothing is pasted yet

    Scenario: Paste at once into an element of a generic kind
      Given the focused element is a terminal's window outside web content
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
      And the overlay shows the Pasted sign with no text window, then hides

    Scenario: Paste once the user clicks into a text input on the web page
      Given the app holds the text back because a web page has focus without a text input
      When the user clicks into a text input on that page
      Then the text is pasted into it
      And the overlay shows the Pasted sign with no text window, then hides

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

  Rule: The focus decision is logged while the focus wait is on

    Scenario: The log names the focused element and the decision
      Given the focus wait is on
      And a button has keyboard focus
      When a dictation finishes
      Then the log names the button's kind and says the app waits for focus

    Scenario: The log says the focused element lies in web content
      Given the focus wait is on
      And a web page has keyboard focus and no text input on it has
      When a dictation finishes
      Then the log names the page's kind, says it lies in web content and says the app waits for focus

  Rule: The paste keeps the user's clipboard

    Scenario: A screenshot copied during the recording survives the paste
      Given the user copies a screenshot to the clipboard while a recording runs
      When a dictation is pasted
      Then the clipboard holds the screenshot again after the paste

    Scenario: Copied text survives the paste
      Given the clipboard holds text the user copied
      When a dictation is pasted
      Then the clipboard holds that text again after the paste

    Scenario: An empty clipboard stays empty
      Given the clipboard is empty
      When a dictation is pasted
      Then the clipboard is empty again after the paste

    Scenario: Quitting right after a paste still restores the clipboard
      Given the clipboard holds text the user copied
      And a dictation was just pasted and its restore delay has not run out
      When the user quits the app
      Then the clipboard holds the user's text again before the app ends

    Scenario: A copy made after the paste is not overwritten
      Given a dictation was just pasted and its restore delay has not run out
      When the user copies something new
      Then the clipboard keeps the new copy after the delay

    Scenario: A second dictation within the delay keeps the user's clipboard
      Given the clipboard holds text the user copied
      And a dictation was just pasted and its restore delay has not run out
      When a second dictation is pasted
      Then the clipboard holds the user's text again after the paste, not the first dictation

  Rule: The transcript is pasted without filler sounds

    Scenario: Filler sounds are dropped from the pasted text
      Given speech recognition returns "Эм, давай, э-э, созвонимся завтра утром."
      When the user dictates and the dictation is pasted
      Then the pasted text is "давай, созвонимся завтра утром."

    Scenario: The transcript reaches the paste unchanged otherwise
      Given speech recognition returns a transcript without filler sounds
      When the user dictates and the dictation is pasted
      Then the pasted text equals the transcript
