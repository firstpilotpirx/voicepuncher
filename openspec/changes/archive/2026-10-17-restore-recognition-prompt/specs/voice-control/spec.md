## ADDED Requirements

### Requirement: Confirmations are not steered by a prompt
The command spotting service SHALL send every confirmation to the speech recognition service without a prompt, so speech that is no command is not written as a command phrase. Measured with the command phrases as the prompt, Russian counting words and short Russian phrases were confirmed as commands and the user's recorded speech gave more wrong commands than without it.

#### Scenario: A confirmation carries no prompt
- **GIVEN** voice control is listening
- **WHEN** an utterance is sent for confirmation
- **THEN** the request to the speech recognition service holds no prompt

#### Scenario: Speech that is no command is not turned into a command
- **GIVEN** voice control is listening
- **WHEN** someone says short Russian phrases ("да", "спасибо", "привет") or Russian counting words, each on its own with a pause around it
- **THEN** no command takes effect
