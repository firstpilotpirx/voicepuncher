## MODIFIED Requirements

### Requirement: A command confirmed slowly under load is still carried out
When the speech recognition service for commands answers its confirmations slowly because the machine is busy, the command spotting service SHALL still report a command said alone or in its place, in the order spoken, within the confirmation's deadline, instead of dropping it as speech without a command.

#### Scenario: A stop right after dictated speech under load
- **GIVEN** voice control runs, a dictation is on, and the speech recognition service for commands takes over a second for every confirmation, as under machine load
- **WHEN** the user dictates a sentence and says "stop dictation" right after its last word
- **THEN** the command spotting service reports "stop dictation"

#### Scenario: A chain of commands under load
- **GIVEN** voice control runs and the speech recognition service for commands takes over a second for every confirmation, as under machine load
- **WHEN** the user says "that is all stop enter start"
- **THEN** the command spotting service reports "stop dictation", "press enter" and "start dictation", in that order

#### Scenario: A restart before a short pause and more speech under load
- **GIVEN** voice control runs, a dictation is on, and the speech recognition service for commands takes over a second for every confirmation, as under machine load
- **WHEN** the user says "restart", pauses for about 0.4 s and says another sentence
- **THEN** the command spotting service reports "restart dictation"
