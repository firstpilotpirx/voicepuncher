## ADDED Requirements

### Requirement: Common developer terms are recognized without configuration
The system SHALL, when the user configures no speech recognition prompt, send a default prompt with every dictation chunk that names common developer terms (shell, tool, service and language names in their usual spelling), so these terms spoken inside Russian or English dictation reach the raw transcript in their usual Latin spelling. A prompt the user configures SHALL replace the default one; a prompt the user configures as empty SHALL turn the prompt off.

#### Scenario: Shell terms inside Russian dictation come out in Latin script
- **GIVEN** the user configured no speech recognition prompt
- **WHEN** the user dictates "Погляди, был ли сохранён конфиг: oh-my-zsh, zsh и Powerlevel10k настройки"
- **THEN** the raw transcript contains "zsh" and "Powerlevel10k" in Latin script and the rest of the sentence in Russian

#### Scenario: The user's own prompt replaces the default one
- **GIVEN** the user configured a speech recognition prompt of their own
- **WHEN** the user dictates a chunk
- **THEN** the chunk is recognized with the user's prompt and without the default one

#### Scenario: An empty prompt turns the prompt off
- **GIVEN** the user configured an empty speech recognition prompt
- **WHEN** the user dictates a chunk
- **THEN** the chunk is recognized without any prompt
