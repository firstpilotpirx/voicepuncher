# Spec Delta

## ADDED Requirements

### Requirement: A dictation lost to another program on the service port names it
When the transcript of a dictation, or a part of it, cannot be produced because no copy of the speech recognition service answers and another program that is no speech recognition service holds the service's own port, the failure notice SHALL name that port and that program instead of telling the user to start the local services.

#### Scenario: Another program holds the dictation port and no copy runs
- **GIVEN** another program that is no speech recognition service listens on the own port of the speech recognition service for dictation, and no copy of the service runs
- **WHEN** the user stops a dictation
- **THEN** nothing is pasted and the overlay shows that the dictation was lost because the port is held by that program, naming the port and the program

#### Scenario: Nothing listens on the dictation port and no copy runs
- **GIVEN** nothing listens on the own port of the speech recognition service for dictation, and no copy of the service runs
- **WHEN** the user stops a dictation
- **THEN** the overlay shows that the dictation was lost because speech recognition failed, and tells the user to start the local services
