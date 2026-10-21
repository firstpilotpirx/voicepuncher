## MODIFIED Requirements

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
