## ADDED Requirements

### Requirement: Confirmations are steered toward the command phrases
The command spotting service SHALL make the speech recognition service use the command phrases as the prompt of every confirmation, so a command said unclearly is confirmed as that command, while speech that is no command is still not confirmed as one.

#### Scenario: The prompt is in effect for a confirmation
- **GIVEN** voice control is listening
- **WHEN** an utterance is sent for confirmation
- **THEN** the speech recognition service decodes it with the command phrases as its prompt

#### Scenario: Speech that is no command is not turned into a command by the prompt
- **GIVEN** voice control is listening
- **WHEN** someone says short Russian phrases ("да", "спасибо", "привет") or Russian counting words, each on its own with a pause around it
- **THEN** no command takes effect
