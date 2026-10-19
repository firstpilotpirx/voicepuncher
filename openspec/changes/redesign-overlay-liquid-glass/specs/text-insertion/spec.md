# Spec Delta

## MODIFIED Requirements

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
