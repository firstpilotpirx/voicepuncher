## MODIFIED Requirements

### Requirement: The icon mirrors the dictation
While the runtime is ready, the icon SHALL mirror the stage of the dictation as the overlay shows it: recording — a filled microphone with a red dot; recognizing speech — a waveform; waiting for a text field — a keyboard; pasted — the ready icon again; a failure notice — a warning triangle for as long as the notice shows, then the ready icon. A command sign SHALL leave the icon as it is.

#### Scenario: A recording shows the recording icon
- **GIVEN** the runtime is ready
- **WHEN** a recording starts
- **THEN** the status item shows the recording icon

#### Scenario: The icon follows every stage of a dictation
- **GIVEN** a button has keyboard focus
- **WHEN** the user records a dictation with speech in it, ends it and then clicks into a text field
- **THEN** the status item shows the recording icon, then the waveform, then the keyboard, then the ready icon after the paste

#### Scenario: A failure notice shows the warning icon until it hides
- **GIVEN** the speech recognition service is not running
- **WHEN** the user stops a dictation
- **THEN** the status item shows the warning icon while the failure notice shows and the ready icon once it hides

#### Scenario: A command sign leaves the icon as it was
- **GIVEN** voice control is turned on and a recording runs
- **WHEN** the user says "clear"
- **THEN** the status item keeps showing the recording icon
