## ADDED Requirements

### Requirement: The transcript is pasted without filler sounds
The system SHALL paste the transcript speech recognition returned, with only its filler sounds (hesitations such as "эм", "э-э", "um", "uh") removed and every other word, its spelling and punctuation kept; a transcript of nothing but filler sounds SHALL be pasted as it was returned. No language model SHALL change the text before it is pasted.

#### Scenario: Filler sounds are dropped from the pasted text
- **GIVEN** speech recognition returns "Эм, давай, э-э, созвонимся завтра утром."
- **WHEN** the dictation is pasted
- **THEN** the pasted text is "давай, созвонимся завтра утром."

#### Scenario: The transcript reaches the paste unchanged otherwise
- **GIVEN** speech recognition returns a transcript without filler sounds
- **WHEN** the dictation is pasted
- **THEN** the pasted text equals the transcript
