# Spec Delta

## Purpose

Defines how the overlay at the top of the screen and its text window look in every state — frosted glass, icons, English labels and motion — so the user reads the app's state at a glance over any window.

## ADDED Requirements

### Requirement: The overlay is frosted glass that stays legible over any window

The overlay pill and the text window under it SHALL be drawn as the system's frosted glass: the window beneath shows through blurred, with rounded corners, a light rim and a soft shadow. Over a dark or saturated window the content SHALL be light; over a light window the glass SHALL turn milky and the content dark. The overlay SHALL take no keyboard focus and no clicks.

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

### Requirement: Every overlay sign has an icon and an English label

Every state of the overlay SHALL show an icon instead of a text glyph, and every label it shows SHALL be in English: "Recognizing", "Refining text", "Click a text field", "Pasted", "Enter", "Clear", "Restart", "CMD" and the failing part's short name. The recording shows no label.

#### Scenario: The processing stages read in English
- **GIVEN** text refinement is enabled
- **WHEN** the user ends a dictation with speech in it
- **THEN** the overlay reads "Recognizing", then "Refining text"

#### Scenario: The recording shows no label
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording
- **THEN** the overlay shows the record sign and the waveform without any text label

### Requirement: The recording shows a pulsing record sign and a scrolling level waveform

While a recording runs, the overlay SHALL show a red record dot inside a ring, pulsing all the time, and a waveform of the microphone level: a new column every 100 ms enters at the right edge while the strip slides smoothly to the left, so the last 3 s of the recording stay visible. Speech SHALL leave tall columns, silence a dotted line; old columns SHALL fade out at the left edge.

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

#### Scenario: The waveform keeps the last three seconds
- **GIVEN** a recording runs for 10 s with speech, then 4 s of silence
- **WHEN** the user looks at the waveform
- **THEN** it shows only dots, the speech having slid out at the left edge

#### Scenario: The record dot pulses in silence too
- **GIVEN** a recording runs
- **WHEN** the user stays silent
- **THEN** the record dot keeps pulsing

### Requirement: The wait for focus shows an orange glass sign with its explanation

While the app holds text back until a text input gets focus, the overlay pill SHALL be orange-tinted glass with a breathing keyboard icon and the label "Click a text field", and the text window SHALL say "Text is ready and pastes once a text field has focus".

#### Scenario: Held-back text shows the orange wait sign
- **GIVEN** a button has keyboard focus
- **WHEN** a dictation finishes
- **THEN** the overlay shows orange glass with a keyboard icon and "Click a text field"
- **AND** the text window says the text pastes once a text field has focus

### Requirement: A pasted text shows the Pasted sign for half a second

Right after the text of a dictation is pasted, the overlay SHALL show a green check, the label "Pasted" and the pasted text in the text window for about 0.5 s, then hide. Any sign that follows SHALL replace it at once. A failure notice about the same dictation (part of it lost, pasted without refinement) SHALL show instead of it. A dictation that pastes nothing — no speech, cancelled, failed, or left in the clipboard after a wait — SHALL NOT show it.

#### Scenario: A pasted dictation shows the Pasted sign
- **GIVEN** a text field has keyboard focus
- **WHEN** a dictation with speech in it finishes
- **THEN** the overlay shows a green check, "Pasted" and the pasted text for about half a second, then hides

#### Scenario: Text pasted after a wait for focus shows the Pasted sign
- **GIVEN** the app holds the text back because a button has focus
- **WHEN** the user clicks into a text field
- **THEN** the overlay shows "Pasted" with the pasted text, then hides

#### Scenario: Text left in the clipboard shows no Pasted sign
- **GIVEN** the app holds the text back because a button has focus
- **WHEN** the user presses the hotkey to end the wait
- **THEN** the overlay hides without showing "Pasted"

#### Scenario: Text pasted without refinement shows the failure notice instead
- **GIVEN** text refinement is on and the text refinement service is not running
- **WHEN** the user stops a dictation and its raw transcript is pasted
- **THEN** the overlay shows the failure notice and not the Pasted sign

#### Scenario: A command right after the paste replaces the Pasted sign
- **GIVEN** voice control is turned on and the overlay shows the Pasted sign
- **WHEN** the user says "press enter"
- **THEN** the enter sign replaces the Pasted sign at once

### Requirement: A command sign shows its icon and name in the glass pill

The enter, clear and restart signs SHALL each show the command's icon and its English name — "Enter", "Clear", "Restart" — in the glass pill, for as long as they show today. The text window SHALL keep what it showed.

#### Scenario: The enter sign shows its icon and name
- **GIVEN** voice control is turned on and no dictation runs
- **WHEN** the user says "press enter"
- **THEN** the overlay shows the return icon and "Enter" for about a second

### Requirement: A failure notice shows an amber glass sign with its message

A failure notice SHALL show in amber-tinted glass a warning triangle and the failing part's short name (STT, REFINE, VOICE, MIC, ACCESS, FAILED) in the pill, and its message, two lines at most, in the text window under it. Over a light window the triangle and the text SHALL turn dark so the notice stays legible.

#### Scenario: A lost dictation shows the amber notice with its message
- **GIVEN** the speech recognition service is not running
- **WHEN** the user stops a dictation
- **THEN** the overlay shows amber glass with a warning triangle and "STT"
- **AND** the text window shows the notice's message

#### Scenario: A failure notice over a white window stays legible
- **GIVEN** a white window lies under the overlay
- **WHEN** a failure notice shows
- **THEN** its triangle and text show dark and legible

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
