# live-transcript Specification

## Purpose
While the user dictates, the overlay shows the text recognized so far, so the user catches a misrecognition early and can re-say or cancel instead of fixing the pasted text.

## Requirements

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
- **GIVEN** voice control is turned on and the user started a dictation by saying "start dictation"
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

### Requirement: The shown text keeps the overlay at a bounded size

When the text recognized so far does not fit the space the overlay gives it, the overlay SHALL show its latest words, marking that earlier words are left out, and SHALL NOT grow beyond that space.

#### Scenario: A long text shows its latest words
- **GIVEN** a running dictation whose text so far is longer than the overlay can show
- **WHEN** the overlay shows that text
- **THEN** it shows the latest words, marked as continuing earlier text
- **AND** the overlay keeps its size

### Requirement: The shown text does not outlive its recording or change the paste

The text recognized so far SHALL disappear when its recording ends — stopped, cancelled or failed — and the next dictation SHALL start with no text shown. Showing it SHALL NOT change the pasted text or delay the paste.

#### Scenario: Stopping a dictation clears its text
- **GIVEN** the overlay shows the text so far of a running dictation
- **WHEN** the user ends the recording
- **THEN** the overlay shows the speech recognition stage without that text
- **AND** the pasted text is the same as without the shown text

#### Scenario: Cancelling a dictation clears its text
- **GIVEN** the overlay shows the text so far of a running dictation
- **WHEN** the user cancels the dictation
- **THEN** the overlay hides with its text

#### Scenario: The next dictation starts without text
- **GIVEN** an earlier dictation showed text while the user spoke
- **WHEN** the user starts a new dictation
- **THEN** the overlay shows the recording sign without any text
