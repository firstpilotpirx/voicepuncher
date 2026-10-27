# dictation-history-window Specification

## Purpose
Let the user find a recent dictation whose text got lost and copy it again, from a window of the menu bar app fed by the dictation archive.

## Requirements

### Requirement: The history window lists the latest dictations newest first
The history window SHALL list the dictations kept in the dictation archive, newest first, one row per dictation, each row showing when the dictation started in local time and its text. The text of a row SHALL be the text to paste, or the raw transcript when the dictation has no text to paste. A dictation with neither SHALL have no row. The window SHALL list at most 300 rows, the newest ones.

#### Scenario: Dictations are listed newest first with their time
- **GIVEN** the archive holds three dictations with text, made one after the other
- **WHEN** the user opens the history window
- **THEN** it lists the three texts, the latest first, each with the local time its dictation started

#### Scenario: A dictation that pasted nothing shows its transcript
- **GIVEN** the archive holds a dictation whose paste failed and one where only the raw transcript is known
- **WHEN** the user opens the history window
- **THEN** the first row shows the text that was to be pasted and the second shows the raw transcript

#### Scenario: A dictation without text has no row
- **GIVEN** the archive holds a dictation on which speech recognition failed and one dictation with text
- **WHEN** the user opens the history window
- **THEN** it lists only the dictation with text

#### Scenario: Only the newest 300 dictations are listed
- **GIVEN** the archive holds 305 dictations with text
- **WHEN** the user opens the history window
- **THEN** it lists 300 rows, the newest 300, and none of the 5 oldest

### Requirement: A row's text is copied to the clipboard
The history window SHALL offer, on each row, a copy action, also reached by double-clicking the row, that puts exactly that row's whole text on the clipboard as plain text, replacing what the clipboard held.

#### Scenario: Copy puts exactly the row's text on the clipboard
- **GIVEN** the history window lists two dictations
- **WHEN** the user copies the second row
- **THEN** the clipboard holds exactly the text of the second row

#### Scenario: Double-click copies the row
- **GIVEN** the history window lists a dictation
- **WHEN** the user double-clicks its row
- **THEN** the clipboard holds exactly the text of that row

### Requirement: The open window shows a new dictation at once
While the history window is open, a dictation archived after it opened SHALL appear as its first row without the user reopening the window, and the list SHALL stay within its 300 rows.

#### Scenario: A new dictation appears while the window is open
- **GIVEN** the history window is open and lists one dictation
- **WHEN** the user dictates a phrase and it is archived
- **THEN** the window lists the new phrase as its first row, above the earlier one

### Requirement: The history window explains an empty list
When there is no row to show, the history window SHALL say why in one line instead of showing an empty table: that the archive is off, or that no dictation has been archived yet.

#### Scenario: The archive is off
- **GIVEN** the user turned the archive off
- **WHEN** the user opens the history window
- **THEN** the window says the archive is off and history is not kept

#### Scenario: Nothing archived yet
- **GIVEN** the archive is on and holds no dictation
- **WHEN** the user opens the history window
- **THEN** the window says no dictation has been kept yet

### Requirement: An unreadable entry does not stop the history window
The history window SHALL skip an archive entry it cannot read (damaged, of an unknown format, or still being written) and list the others; for a damaged entry or one of an unknown format it SHALL log a warning naming the skipped entry, and an entry still being written is skipped without a warning.

#### Scenario: A damaged entry is skipped
- **GIVEN** the archive holds two dictations with text and one damaged entry between them
- **WHEN** the user opens the history window
- **THEN** it lists the two dictations and the log warns about the damaged entry
