# Spec Delta

## MODIFIED Requirements

### Requirement: The first recognizer to hear a command reports it once
With the streaming recognizer, a command SHALL be reported by whichever recognizer hears it first; the same command heard by the other recognizer in the same utterance SHALL NOT be reported again, however far apart the two recognizers time it: the n-th time one recognizer hears a command in an utterance is the utterance's n-th, whatever its stamps. A doubled command ("stop stop") SHALL count once, and the same command said again after a pause in the utterance SHALL count again.

#### Scenario: A command both recognizers hear is reported once
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "enter" followed by silence
- **THEN** the service reports "press enter" exactly once

#### Scenario: A doubled command is reported once
- **GIVEN** the command spotting service runs with the streaming recognizer
- **WHEN** an app streams "enter enter" followed by silence
- **THEN** the service reports "press enter" exactly once

#### Scenario: A command the recognizers time a second apart is reported once
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** the English recognizer times the words it hears about a second later than the Russian one
- **WHEN** an app streams "enter" followed by silence, so the Russian recognizer reports it at the pause and the English one only near the close
- **THEN** the service reports "press enter" exactly once

#### Scenario: A command said twice with a pause between is reported twice by both recognizers
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** the English recognizer times the words it hears about a second later than the Russian one
- **WHEN** an app streams "enter", a pause the gate misses, and "enter" again, both heard by both recognizers
- **THEN** the service reports "press enter" exactly twice
