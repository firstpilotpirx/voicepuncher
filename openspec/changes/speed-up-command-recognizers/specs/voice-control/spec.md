## ADDED Requirements

### Requirement: Streaming commands keep their bounds while every core is busy
With the streaming recognizer and every core of the machine busy with other work, the command spotting service SHALL report "press enter" ending a sentence within 0.6 s of the end of its words (wall clock), as on an idle machine.

#### Scenario: Press enter after a sentence is reported in time with every core busy
- **GIVEN** the command spotting service runs with the streaming recognizer
- **AND** every core of the machine is busy with other work
- **WHEN** an app streams a sentence ending with "press enter", followed by silence, in real time
- **THEN** the service reports "press enter" within 0.6 s of the end of "press enter" by the wall clock
