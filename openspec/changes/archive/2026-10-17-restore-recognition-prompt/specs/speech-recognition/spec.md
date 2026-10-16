## ADDED Requirements

### Requirement: The configured prompt steers recognition
The system SHALL make the speech recognition service use the configured `[transcription] prompt` for every 30-second window of every dictation chunk, ahead of the text recognized in earlier windows of the same chunk, so domain terms named in the prompt are recognized as written there. The last piece of a chunk shorter than 5 seconds is the exception: whisper.cpp decodes it without any earlier text, the prompt included. With an empty prompt, recognition SHALL be the same as without one.

#### Scenario: A domain term named in the prompt is recognized
- **GIVEN** the configured prompt names a domain term that speech recognition misses without the prompt
- **WHEN** the user dictates a sentence with that term
- **THEN** the raw transcript contains the term as written in the prompt

#### Scenario: The prompt holds in every window of a long chunk
- **GIVEN** the configured prompt names a domain term
- **WHEN** the user dictates a chunk longer than 30 seconds that says the term after the first 30 seconds and goes on speaking after it
- **THEN** the raw transcript contains the term as written in the prompt

### Requirement: Text of one request never steers another
The system SHALL keep the text recognized for one request out of every other request. Within a chunk sent with a prompt, the text recognized in earlier windows MAY steer the next window after the prompt; without a prompt, text recognized in one window SHALL NOT be carried into the next.

#### Scenario: A dictation does not echo the previous one
- **GIVEN** the user dictated a sentence and it was transcribed
- **WHEN** the user dictates an unrelated sentence
- **THEN** the raw transcript of the second dictation contains none of the words of the first that were not spoken again
