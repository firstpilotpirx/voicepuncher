## MODIFIED Requirements

### Requirement: Known recognition hallucinations are discarded
The system SHALL discard a transcript that consists only of a known recognition hallucination (subtitle credits, subscription calls, blank-audio markers, a phrase repeated in a loop, the word "you" alone or repeated, ignoring case and punctuation) instead of pasting it.

#### Scenario: Silence produces a subtitle credit
- **WHEN** the user records silence and the service returns a subtitle credit line
- **THEN** nothing is pasted

#### Scenario: Silence produces a lone "you"
- **GIVEN** a recording of silence lasting 1, 10 or 60 seconds
- **AND** the service detects the spoken language automatically
- **WHEN** the user transcribes the recording
- **THEN** no text is produced

#### Scenario: Speech containing "you" is kept
- **WHEN** the service returns a transcript in which "you" appears together with other words
- **THEN** the transcript is passed on for refinement and pasting
