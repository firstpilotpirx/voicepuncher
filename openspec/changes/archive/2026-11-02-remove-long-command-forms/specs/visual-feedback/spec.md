## MODIFIED Requirements

### Requirement: A command sign shows its icon and name in the glass pill

The enter, clear, paste and restart signs SHALL each show the command's icon and its English name — "Enter", "Clear", "Paste", "Restart" — in the glass pill, for as long as they show today. The text window SHALL keep what it showed.

#### Scenario: The enter sign shows its icon and name
- **GIVEN** voice control is turned on and no dictation runs
- **WHEN** the user says "enter"
- **THEN** the overlay shows the return icon and "Enter" for about a second

### Requirement: Pasted text shows the Pasted sign for half a second

Right after the text of a dictation is pasted, the overlay SHALL show a green check and the label "Pasted", with no text window, for about 0.5 s, then hide. Any sign that follows SHALL replace it at once. A failure notice about the same dictation (part of it lost) SHALL show instead of it. A dictation that pastes nothing — no speech, cancelled, failed, or left in the clipboard after a wait — SHALL NOT show it.

#### Scenario: A pasted dictation shows the Pasted sign
- **GIVEN** a text field has keyboard focus
- **WHEN** a dictation with speech in it finishes
- **THEN** the overlay shows a green check and "Pasted" for about half a second, then hides
- **AND** no text window with the pasted text shows under it

#### Scenario: Text pasted after a wait for focus shows the Pasted sign
- **GIVEN** the app holds the text back because a button has focus
- **WHEN** the user clicks into a text field
- **THEN** the overlay shows "Pasted" without a text window, then hides

#### Scenario: Text left in the clipboard shows no Pasted sign
- **GIVEN** the app holds the text back because a button has focus
- **WHEN** the user presses the hotkey to end the wait
- **THEN** the overlay hides without showing "Pasted"

#### Scenario: A command right after the paste replaces the Pasted sign
- **GIVEN** voice control is turned on and the overlay shows the Pasted sign
- **WHEN** the user says "enter"
- **THEN** the enter sign replaces the Pasted sign at once
