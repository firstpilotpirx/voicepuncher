## MODIFIED Requirements

### Requirement: The overlay shows the text recognized so far while the user dictates

While a recording runs and is transcribed in chunks, the overlay SHALL show, next to the recording sign, the raw transcript of every chunk transcribed so far, stitched at the seams as the pasted transcript is. It SHALL update shortly after each chunk is transcribed, the same for a dictation started by the hotkey and one started by voice. Before the first chunk is transcribed the overlay SHALL show the recording sign alone.

#### Scenario: A hotkey dictation shows its first part once it is transcribed
- **GIVEN** the user records a dictation with the hotkey
- **WHEN** the first part of the recording is transcribed while the user speaks
- **THEN** the overlay shows the text of that part next to the recording sign

#### Scenario: The shown text grows with every transcribed part
- **GIVEN** the overlay shows the text of the first part of a running dictation
- **WHEN** the next part is transcribed
- **THEN** the overlay shows the text of both parts joined, with their shared words once

#### Scenario: A dictation started by voice shows its text so far
- **GIVEN** voice control is turned on and the user started a dictation by saying "start"
- **WHEN** the first part of the recording is transcribed while the user speaks
- **THEN** the overlay shows the text of that part next to the recording sign

#### Scenario: A short dictation shows the recording sign alone
- **GIVEN** the user records a dictation shorter than one part
- **WHEN** the recording runs
- **THEN** the overlay shows the recording sign without any text

#### Scenario: A part without speech adds no text
- **GIVEN** the overlay shows the text of the first part of a running dictation
- **WHEN** the next part holds no speech and is transcribed
- **THEN** the overlay keeps showing the same text
