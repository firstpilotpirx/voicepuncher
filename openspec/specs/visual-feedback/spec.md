# visual-feedback Specification

## Purpose
Defines how the overlay at the top of the screen and its text window look in every state — frosted glass, icons, English labels and motion — so the user reads the app's state at a glance over any window.

## Requirements

### Requirement: The overlay is frosted glass that stays legible over any window

The overlay pill and the text window under it SHALL be drawn as the system's clear frosted glass: clearly translucent, the window beneath showing through blurred, with rounded corners, an even edge with no brighter line along any side, and a soft shadow. Over a dark or saturated window the content SHALL be light; over a light window the glass SHALL turn milky and the content dark. The overlay SHALL take no keyboard focus and no clicks.

#### Scenario: The overlay over a dark window shows light content
- **GIVEN** a dark or saturated window lies under the overlay
- **WHEN** the user starts a recording
- **THEN** the overlay shows as blurred glass with light icons and text

#### Scenario: The overlay over a light window shows dark content
- **GIVEN** a white window lies under the overlay
- **WHEN** the user starts a recording
- **THEN** the overlay shows as milky glass with dark icons and text, all of them legible

#### Scenario: The glass overlay takes no focus
- **GIVEN** a text field of another app has keyboard focus
- **WHEN** the overlay shows any state
- **THEN** the text field keeps the keyboard focus

#### Scenario: The window under the overlay shows through the glass
- **GIVEN** a window with a bright picture lies under the overlay
- **WHEN** the user starts a recording
- **THEN** the picture shows through the pill, blurred, instead of the pill looking solid

#### Scenario: The glass has no bright line along its top edge
- **GIVEN** no dictation runs
- **WHEN** the overlay shows the recording, a processing stage or the Pasted sign
- **THEN** the pill's top edge looks like the rest of its edge, with no brighter straight line along it

### Requirement: The wait for focus shows an orange glass sign with its explanation

While the app holds text back until a text input gets focus, the overlay pill SHALL be orange-tinted glass with a breathing keyboard icon and the label "Click a text field", and SHALL show no text window.

#### Scenario: Held-back text shows the orange wait sign
- **GIVEN** a button has keyboard focus
- **WHEN** a dictation finishes
- **THEN** the overlay shows orange glass with a keyboard icon and "Click a text field"
- **AND** no text window shows under it

### Requirement: A command sign shows its icon and name in the glass pill

The enter, clear, paste and restart signs SHALL each show the command's icon and its English name — "Enter", "Clear", "Paste", "Restart" — in the glass pill, for as long as they show today. The text window SHALL keep what it showed.

#### Scenario: The enter sign shows its icon and name
- **GIVEN** voice control is turned on and no dictation runs
- **WHEN** the user says "press enter"
- **THEN** the overlay shows the return icon and "Enter" for about a second

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

### Requirement: The commands-unavailable sign is a small grey glass pill

When the overlay shows that voice commands do not work, it SHALL show a microphone-off icon with "CMD" in a small grey glass pill alone in standby, and the microphone-off icon without a label after a thin separator beside the recording, processing or waiting sign; the waveform then gets narrower so the pill keeps its size.

#### Scenario: Standby shows the small CMD pill
- **GIVEN** listening is on and no dictation runs
- **WHEN** the command spotting service stays away for longer than 5 seconds
- **THEN** the overlay shows a small grey pill with a microphone-off icon and "CMD"

#### Scenario: A recording shows the microphone-off icon after the waveform
- **GIVEN** a recording runs
- **WHEN** the command spotting service stays away for longer than 5 seconds
- **THEN** the overlay shows the microphone-off icon after a separator to the right of the waveform

### Requirement: Reduce Motion stops the decorative motion

When the user turns on Reduce Motion in macOS, the overlay SHALL stop the record dot's pulse, the icons' breathing and the text window's rise-in; the spinners SHALL still turn and the waveform SHALL still update, one column per step instead of sliding.

#### Scenario: Reduce Motion keeps the record dot still
- **GIVEN** Reduce Motion is on
- **WHEN** the user starts a recording and speaks
- **THEN** the record dot does not pulse
- **AND** the waveform still adds a column for the speech

### Requirement: The recording shows a pulsing record sign and a slow scrolling level waveform

While a recording runs, the overlay SHALL show a red record dot inside a ring, pulsing all the time, and a waveform of the microphone level: a new column every 200 ms enters at the right edge while the strip slides smoothly to the left, at half the pace it had before, so the last 6 s of the recording stay visible. Speech SHALL leave tall columns, silence a dotted line; old columns SHALL fade out at the left edge. The waveform SHALL show in the same frame as the record sign, never as an empty strip before its first update.

#### Scenario: Speech leaves tall columns that slide to the left
- **GIVEN** a recording runs
- **WHEN** the user speaks for a second
- **THEN** tall columns enter the waveform at its right edge and slide to the left

#### Scenario: Silence leaves a dotted line
- **GIVEN** a recording runs
- **WHEN** the user stays silent for a second
- **THEN** the waveform's newest part is a row of dots that slides to the left

#### Scenario: A new recording starts with a dotted waveform
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording
- **THEN** the waveform shows only dots until the user speaks

#### Scenario: The waveform shows together with the record sign
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording
- **THEN** the record sign and the dotted waveform appear in the same frame, with no empty waveform before them

#### Scenario: The waveform keeps the last six seconds
- **GIVEN** a recording runs for 10 s with speech, then 7 s of silence
- **WHEN** the user looks at the waveform
- **THEN** it shows only dots, the speech having slid out at the left edge

#### Scenario: The waveform moves at half its former pace
- **GIVEN** a recording runs
- **WHEN** the user speaks for one second
- **THEN** five new columns enter the waveform, half as many as before, and the strip slides half as far

#### Scenario: The record dot pulses in silence too
- **GIVEN** a recording runs
- **WHEN** the user stays silent
- **THEN** the record dot keeps pulsing

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
