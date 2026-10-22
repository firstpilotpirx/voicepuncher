## MODIFIED Requirements

### Requirement: The paste keeps the user's clipboard

When the system pastes a dictation, the clipboard SHALL afterwards hold again what it held right before the paste, whatever its kind — text, an image, files or several kinds at once — including content the user copied while the recording ran. An empty clipboard SHALL stay empty. When the app quits, or its pipeline stops, before the restore delay ran out, the clipboard SHALL be restored at once rather than lost.

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
