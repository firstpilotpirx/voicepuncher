# Spec Delta

## ADDED Requirements

### Requirement: A confirmation never waits past its deadline for a copy
The command spotting service SHALL send a confirmation to a free copy of the speech recognition service for commands, or wait for one only until the confirmation's deadline; a confirmation that finds no free copy by then SHALL be given up and logged without ever being sent, so given-up confirmations never occupy a copy.

#### Scenario: Every copy for commands is busy past the deadline
- **GIVEN** voice control runs, and every copy of the speech recognition service for commands is busy for longer than a confirmation's deadline
- **WHEN** the user says "press enter"
- **THEN** that confirmation is logged as given up, no copy receives it, and listening goes on

#### Scenario: A copy frees before the deadline
- **GIVEN** voice control runs, and every copy of the speech recognition service for commands is busy for one more second
- **WHEN** the user says "press enter"
- **THEN** the confirmation is sent to the first copy that frees, and "press enter" is reported
