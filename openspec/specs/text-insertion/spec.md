# text-insertion Specification

## Purpose
Delivers the final text of a dictation into the user's app: pasted at once into the focused text input, held back until a text input gets focus when nothing that accepts text is focused, and left in the clipboard when that wait ends without one.

## Requirements

### Requirement: Text is pasted at once unless no text input is surely focused
The system SHALL paste the final text at once whenever a text input may have keyboard focus. It SHALL hold the text back only when the focused element is either a clearly non-text control (button, list, table, outline, image, menu, check box, radio button, slider, tab group, toolbar) that offers neither a text selection nor an editable value, or an element of web content of a browser or an Electron app (the page itself or an element inside it) that is not a text input (text field, text area, combo box, search field) and offers no editable value. A text selection on a page does not make it a text input. When the system cannot tell what has focus, or the focused element is of any other kind outside web content, it SHALL paste at once. Before it reads the focus, the system SHALL ask the focused app to expose its web content to the Accessibility API, so a text input on a page is reported as a text input.

#### Scenario: Paste at once into a focused text field
- **GIVEN** a text field has keyboard focus
- **WHEN** a dictation finishes
- **THEN** the text is pasted at once
- **AND** the overlay never shows that it waits for focus

#### Scenario: Paste at once when the app does not tell what has focus
- **GIVEN** the focused app does not report its focused element
- **WHEN** a dictation finishes
- **THEN** the text is pasted at once

#### Scenario: Paste at once into a text input on a web page
- **GIVEN** a text input on a page of a browser or an Electron app has keyboard focus
- **WHEN** a dictation finishes
- **THEN** the text is pasted at once

#### Scenario: Paste at once into an editable element on a web page
- **GIVEN** an element on a web page that offers an editable value has keyboard focus
- **WHEN** a dictation finishes
- **THEN** the text is pasted at once

#### Scenario: Hold the text back when a web page has focus without a text input
- **GIVEN** a page of a browser or an Electron app has keyboard focus and no text input on it has
- **WHEN** a dictation finishes
- **THEN** nothing is pasted yet
- **AND** the overlay shows that the app waits for focus

#### Scenario: Hold the text back when text is selected on a web page
- **GIVEN** a page of a browser or an Electron app has keyboard focus with some of its text selected and no text input on it has focus
- **WHEN** a dictation finishes
- **THEN** nothing is pasted yet

#### Scenario: Paste at once into an element of a generic kind
- **GIVEN** the focused element is a window or a group outside web content, as a terminal reports it
- **WHEN** a dictation finishes
- **THEN** the text is pasted at once

#### Scenario: Paste at once into a list that offers a text selection
- **GIVEN** the focused element is a list that offers a text selection
- **WHEN** a dictation finishes
- **THEN** the text is pasted at once

#### Scenario: Hold the text back when a button has focus
- **GIVEN** a button has keyboard focus
- **WHEN** a dictation finishes
- **THEN** nothing is pasted yet
- **AND** the overlay shows that the app waits for focus

### Requirement: Held-back text is pasted when a text input gets focus
The system SHALL keep checking the focus while it holds the text back, about every 100 ms, and SHALL paste the text as soon as the rule above allows a paste. The overlay SHALL show the waiting state for the whole wait, then the Pasted sign once the text is pasted, and then hide.

#### Scenario: Paste once the user clicks into a text field
- **GIVEN** the app holds the text back because a button has focus
- **WHEN** the user clicks into a text field
- **THEN** the text is pasted into it
- **AND** the overlay shows the Pasted sign, then hides

#### Scenario: Paste once the user clicks into a text input on the web page
- **GIVEN** the app holds the text back because a web page has focus without a text input
- **WHEN** the user clicks into a text input on that page
- **THEN** the text is pasted into it
- **AND** the overlay shows the Pasted sign, then hides

### Requirement: A wait that ends without a text input leaves the text in the clipboard
The system SHALL end the wait when the configured wait time passes or the user presses the hotkey. It SHALL then put the text in the clipboard without pasting it, keep it there, hide the overlay and log that the text is in the clipboard. The hotkey press that ends a wait SHALL NOT start a recording; the next press SHALL.

#### Scenario: The wait time passes without a text input
- **GIVEN** the app holds the text back because a button has focus
- **WHEN** the wait time passes and the button still has focus
- **THEN** nothing is pasted
- **AND** the clipboard holds the text
- **AND** the overlay hides

#### Scenario: A hotkey press ends the wait
- **GIVEN** the app holds the text back because a button has focus
- **WHEN** the user presses the hotkey
- **THEN** nothing is pasted
- **AND** the clipboard holds the text
- **AND** no recording starts

#### Scenario: The next hotkey press after an ended wait records
- **GIVEN** a hotkey press ended a wait for focus
- **WHEN** the user presses the hotkey again
- **THEN** a recording starts

### Requirement: The wait time is a setting
The system SHALL read the wait time in seconds from the config file and SHALL wait 5 minutes when the setting is absent. A wait time of 0 SHALL turn the wait off: the text is pasted at once whatever has focus.

#### Scenario: Default wait time is five minutes
- **GIVEN** the config file does not set the wait time
- **WHEN** the app starts
- **THEN** the app waits up to 5 minutes for a text input

#### Scenario: A wait time of zero pastes at once
- **GIVEN** the config file sets the wait time to 0
- **AND** a button has keyboard focus
- **WHEN** a dictation finishes
- **THEN** the text is pasted at once

### Requirement: The focus decision is logged while the focus wait is on
Whenever the focus wait is on, the system SHALL log, for every dictation it delivers, the kind of the focused element it saw (or that it could not tell), whether it lies in web content, and whether it pasted at once or waited.

#### Scenario: The log names the focused element and the decision
- **GIVEN** a button has keyboard focus
- **WHEN** a dictation finishes
- **THEN** the log names the button's kind and says the app waits for focus

#### Scenario: The log says the focused element lies in web content
- **GIVEN** a page of a browser or an Electron app has keyboard focus and no text input on it has
- **WHEN** a dictation finishes
- **THEN** the log names the page's kind, says it lies in web content and says the app waits for focus

### Requirement: The paste keeps the user's clipboard

When the system pastes a dictation, the clipboard SHALL afterwards hold again what it held right before the paste, whatever its kind — text, an image, files or several kinds at once — including content the user copied while the recording ran. An empty clipboard SHALL stay empty. What the user copies after the paste SHALL stay; a second paste within the restore delay SHALL still leave the user's clipboard, not the first dictation. When the app quits, or its pipeline stops, before the restore delay ran out, the clipboard SHALL be restored at once rather than lost.

#### Scenario: A screenshot copied during the recording survives the paste
- **GIVEN** a text field has keyboard focus and a recording runs
- **AND** the user copies a screenshot to the clipboard
- **WHEN** the user ends the recording by saying "stop dictation" and the text is pasted
- **THEN** the clipboard holds the screenshot again after the paste

#### Scenario: Copied text survives the paste
- **GIVEN** the clipboard holds text the user copied
- **WHEN** a dictation is pasted
- **THEN** the clipboard holds that text again after the paste

#### Scenario: An empty clipboard stays empty
- **GIVEN** the clipboard is empty
- **WHEN** a dictation is pasted
- **THEN** the clipboard is empty again after the paste

#### Scenario: Quitting right after a paste still restores the clipboard
- **GIVEN** the clipboard holds text the user copied
- **AND** a dictation was just pasted and its restore delay has not run out
- **WHEN** the user quits the app
- **THEN** the clipboard holds the user's text again before the app ends

#### Scenario: A copy made after the paste is not overwritten
- **GIVEN** a dictation was just pasted and its restore delay has not run out
- **WHEN** the user copies something new
- **THEN** the clipboard keeps the new copy after the delay

#### Scenario: A second dictation within the delay keeps the user's clipboard
- **GIVEN** the clipboard holds text the user copied
- **AND** a dictation was just pasted and its restore delay has not run out
- **WHEN** a second dictation is pasted
- **THEN** the clipboard holds the user's text again after the paste, not the first dictation
