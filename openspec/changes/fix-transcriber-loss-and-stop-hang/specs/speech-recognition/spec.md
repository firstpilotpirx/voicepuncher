## ADDED Requirements

### Requirement: A dead transcription worker loses only its dictation
The system SHALL end a dictation whose live transcription worker stopped without a result with the notice that speech recognition failed, and SHALL transcribe and paste the next dictation as usual; no dictation SHALL find the transcriber gone.

#### Scenario: The worker dies mid-dictation
- **GIVEN** the live transcription worker of a dictation stops without a result
- **WHEN** the user finishes that dictation and then dictates again
- **THEN** the first dictation ends with the notice that speech recognition failed
- **AND** the text of the second dictation is pasted

### Requirement: Stopping a dictation waits a bounded time for its transcript
The system SHALL wait for the transcript of a stopped dictation no longer than the longest time speech recognition may take on one request with its retries, plus two seconds; past it the dictation SHALL end with the notice that speech recognition failed, the recording SHALL stop, and the app SHALL handle the hotkey again, including a press made during the wait.

#### Scenario: The speech recognition request never answers
- **GIVEN** the speech recognition service never answers the last chunk of a dictation
- **WHEN** the user finishes the dictation and presses the hotkey during the wait
- **THEN** the dictation ends with the notice that speech recognition failed once the wait is over
- **AND** the hotkey press made during the wait starts a new recording
