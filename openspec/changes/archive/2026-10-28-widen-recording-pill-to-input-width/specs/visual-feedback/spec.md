## MODIFIED Requirements

### Requirement: The recording shows a pulsing record sign and a slow scrolling level waveform

While a recording runs, the overlay SHALL show a red record dot inside a ring, pulsing all the time, and a waveform of the microphone level: a new column every 110 ms enters at the right edge while the strip slides smoothly to the left, so the last 23.1 s of the recording stay visible, which makes the pill 922 pt long, as long as the chat input box of the Claude desktop app. Silence and the room's noise SHALL leave a dotted line; the user's speech SHALL leave columns whose heights follow how loudly they speak, quiet speech visible and loud speech taller, from the room's noise floor (about -62 dBFS) to the user's loud speech (about -28 dBFS); old columns SHALL fade out at the left edge. The waveform SHALL show in the same frame as the record sign, never as an empty strip before its first update. The pill's length SHALL stay fixed: it SHALL NOT follow the size of the Claude window or of any other window.

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

#### Scenario: The waveform still shows speech from 22.5 seconds ago
- **GIVEN** a recording runs with a second of speech
- **WHEN** the user stays silent for 22.5 s
- **THEN** the waveform's oldest column is still that speech

#### Scenario: The waveform keeps the last six seconds
- **GIVEN** a recording runs for 20 s with speech, then 24 s of silence
- **WHEN** the user looks at the waveform
- **THEN** it shows only dots, the speech having slid out at the left edge

#### Scenario: The waveform moves at half its former pace
- **GIVEN** a recording runs
- **WHEN** the user speaks for one second
- **THEN** nine new columns enter the waveform

#### Scenario: The record dot pulses in silence too
- **GIVEN** a recording runs
- **WHEN** the user stays silent
- **THEN** the record dot keeps pulsing

#### Scenario: The recording pill is as long as ChatGPT's dictation bar
- **GIVEN** no dictation runs
- **WHEN** the user starts a recording
- **THEN** the pill is 922 pt wide, with the waveform's 23.1 s of columns spanning it
