## MODIFIED Requirements

### Requirement: A failure notice shows an amber glass sign with its message

A failure notice SHALL show in amber-tinted glass a warning triangle and the failing part's short name (STT, VOICE, MIC, ACCESS, FAILED) in the pill, with no text window; its message SHALL go to the log. Over a light window the triangle and the label SHALL turn dark so the notice stays legible.

#### Scenario: A lost dictation shows the amber notice with its message
- **GIVEN** the speech recognition service is not running
- **WHEN** the user stops a dictation
- **THEN** the overlay shows amber glass with a warning triangle and "STT"
- **AND** no text window shows under it
- **AND** the log holds the notice's message

#### Scenario: A failure notice over a white window stays legible
- **GIVEN** a white window lies under the overlay
- **WHEN** a failure notice shows
- **THEN** its triangle and label show dark and legible

### Requirement: The text window shows only during a recording

The text window under the pill SHALL show only while a recording runs, holding the text recognized so far; it SHALL hang below the pill, its top edge under the pill's bottom edge with a small gap, and SHALL never be covered by the pill. Every other state — speech recognition, the wait for focus, Pasted, a command sign after the recording, a failure notice, the commands-unavailable sign alone — SHALL show no text window.

#### Scenario: The text recognized so far shows below the pill
- **GIVEN** a recording runs and its first part has been transcribed
- **WHEN** the user looks at the overlay
- **THEN** the text of that part shows in full in a window below the pill, not covered by it

#### Scenario: Ending the recording takes the text window away
- **GIVEN** the overlay shows the text recognized so far under the recording sign
- **WHEN** the user ends the recording
- **THEN** the text window hides and stays hidden through speech recognition and Pasted

### Requirement: The pill keeps one size in every state

The overlay pill SHALL have the same width and height in every state it shows — the start of a recording, the recording with its waveform, speech recognition, the wait for focus, Pasted, a command sign and a failure notice — with its content centred in it; only the commands-unavailable sign alone in standby MAY be smaller.

#### Scenario: The pill does not change size from the recording to Pasted
- **GIVEN** a text field has keyboard focus
- **WHEN** the user records a dictation with speech in it and it is pasted
- **THEN** the pill has the same width and height at the start of the recording, while recording, while recognizing and while showing "Pasted"

## ADDED Requirements

### Requirement: Each overlay sign shows an icon and an English label

Every state of the overlay SHALL show an icon instead of a text glyph, and every label it shows SHALL be in English: "Recognizing", "Click a text field", "Pasted", "Enter", "Clear", "Paste", "Restart", "CMD" and the failing part's short name. The recording shows no label.

#### Scenario: The processing stage reads in English
- **GIVEN** the user records a dictation with speech in it
- **WHEN** the user ends the recording
- **THEN** the overlay reads "Recognizing"

#### Scenario: The recording shows no label
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording
- **THEN** the overlay shows the record sign and the waveform without any text label

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
- **WHEN** the user says "press enter"
- **THEN** the enter sign replaces the Pasted sign at once

## REMOVED Requirements

### Requirement: Every overlay sign has an icon and an English label

**Reason**: Text refinement is removed; the requirement is restated without it as "Each overlay sign shows an icon and an English label".
**Migration**: None.

### Requirement: A pasted text shows the Pasted sign for half a second

**Reason**: Text refinement is removed; the requirement is restated without it as "Pasted text shows the Pasted sign for half a second".
**Migration**: None.
