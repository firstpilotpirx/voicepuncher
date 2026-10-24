## MODIFIED Requirements

### Requirement: The overlay is frosted glass that stays legible over any window

The overlay pill and the text window under it SHALL be drawn as the system's clear frosted glass: clearly translucent, the window beneath showing through blurred, with rounded corners, an even edge with no brighter line along any side, and a soft shadow. The pill's corners SHALL follow the continuous curve of macOS notifications, a superellipse whose corner is well under half the pill's height, never a capsule with half-circle ends; its edge SHALL be a thin, faint hairline, the same all around. Over a dark or saturated window the content SHALL be light; over a light window the glass SHALL turn milky and the content dark. The overlay SHALL take no keyboard focus and no clicks.

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

#### Scenario: The pill has notification corners, not round ends
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording
- **THEN** the pill's corners are continuous curves well under half its height, and a faint hairline of the same strength runs all around its edge

### Requirement: The recording shows a pulsing record sign and a slow scrolling level waveform

While a recording runs, the overlay SHALL show a red record dot inside a ring, pulsing all the time, and a waveform of the microphone level: a new column every 100 ms enters at the right edge while the strip slides smoothly to the left, so the last 9 s of the recording stay visible, which makes the pill about as long as ChatGPT's voice input bar. Silence and the room's noise SHALL leave a dotted line; the user's speech SHALL leave columns whose heights follow how loudly they speak, quiet speech visible and loud speech taller, from the room's noise floor (about -62 dBFS) to the user's loud speech (about -28 dBFS); old columns SHALL fade out at the left edge. The waveform SHALL show in the same frame as the record sign, never as an empty strip before its first update.

#### Scenario: Speech leaves tall columns that slide to the left
- **GIVEN** a recording runs
- **WHEN** the user speaks for a second
- **THEN** tall columns enter the waveform at its right edge and slide to the left

#### Scenario: Silence leaves a dotted line
- **GIVEN** a recording runs
- **WHEN** the user stays silent for a second, in the noise of their room
- **THEN** the waveform's newest part is a row of dots that slides to the left

#### Scenario: Quiet and loud speech leave columns of different heights
- **GIVEN** a recording runs
- **WHEN** the user speaks quietly, then loudly
- **THEN** the quiet speech leaves columns clearly taller than dots, and the loud speech columns clearly taller than the quiet ones

#### Scenario: A new recording starts with a dotted waveform
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording
- **THEN** the waveform shows only dots until the user speaks

#### Scenario: The waveform shows together with the record sign
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording
- **THEN** the record sign and the dotted waveform appear in the same frame, with no empty waveform before them

#### Scenario: The waveform keeps the last six seconds
- **GIVEN** a recording runs for 10 s with speech, then 10 s of silence
- **WHEN** the user looks at the waveform
- **THEN** it shows only dots, the speech having slid out at the left edge

#### Scenario: The waveform moves at half its former pace
- **GIVEN** a recording runs
- **WHEN** the user speaks for one second
- **THEN** ten new columns enter the waveform

#### Scenario: The record dot pulses in silence too
- **GIVEN** a recording runs
- **WHEN** the user stays silent
- **THEN** the record dot keeps pulsing
