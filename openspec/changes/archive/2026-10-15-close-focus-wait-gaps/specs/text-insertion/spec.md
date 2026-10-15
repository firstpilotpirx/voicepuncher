## MODIFIED Requirements

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
The system SHALL keep checking the focus while it holds the text back, about every 100 ms, and SHALL paste the text as soon as the rule above allows a paste. The overlay SHALL show the waiting state for the whole wait and hide once the text is pasted.

#### Scenario: Paste once the user clicks into a text field
- **GIVEN** the app holds the text back because a button has focus
- **WHEN** the user clicks into a text field
- **THEN** the text is pasted into it
- **AND** the overlay hides

#### Scenario: Paste once the user clicks into a text input on the web page
- **GIVEN** the app holds the text back because a web page has focus without a text input
- **WHEN** the user clicks into a text input on that page
- **THEN** the text is pasted into it
- **AND** the overlay hides

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
