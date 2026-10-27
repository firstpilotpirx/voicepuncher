## ADDED Requirements

### Requirement: A command spotting copy of another protocol version is replaced
When the app manages its services and finds a copy of the command spotting service already answering on one of its ports, it SHALL check which protocol version that copy speaks before adopting it. A copy speaking another version than the app SHALL NOT be adopted: the app SHALL stop it and start a copy of its own on the same port, and the log SHALL name the replacement once, with the port and both versions. A program on that port that is not the command spotting service SHALL NOT be stopped.

#### Scenario: A copy left by an older app is replaced at launch
- **GIVEN** a command spotting service of an older protocol version runs on a copy port, left by an earlier app
- **WHEN** the app launches with voice control on
- **THEN** that copy is stopped and the app starts its own copy on the same port
- **AND** the log names the port, the old and the new protocol version in one line
- **AND** a spoken command is carried out without any manual step

#### Scenario: A copy of the same protocol version is adopted as before
- **GIVEN** a command spotting service of the app's protocol version runs on a copy port
- **WHEN** the app launches with voice control on
- **THEN** the app adopts it and starts nothing on that port

#### Scenario: Another program refusing the app's protocol is not stopped
- **GIVEN** a program that is not the command spotting service holds a command spotting port and refuses the app's protocol version
- **WHEN** the app launches with voice control on
- **THEN** the program keeps running and the service on that port is Failed with a reason naming the port

### Requirement: A protocol refusal while voice control runs replaces the copy
When the app manages its services and an adopted copy of the command spotting service refuses voice control because it speaks another protocol version, the app SHALL check that copy again and replace it as at launch, instead of reconnecting to it over and over.

#### Scenario: A stale copy appears after the app started
- **GIVEN** the app runs with voice control on and an adopted copy of the command spotting service is replaced from outside by one of an older protocol version
- **WHEN** voice control connects to that copy and is refused for its protocol version
- **THEN** the app stops that copy and starts its own on the same port
- **AND** voice control works again without any manual step

### Requirement: Voice control passes over a copy of another protocol version
When voice control connects to the command spotting service and a copy refuses it for its protocol version, voice control SHALL connect to the next copy instead, so commands keep working while the refused copy is replaced.

#### Scenario: One of two copies speaks an older protocol version
- **GIVEN** two copies of the command spotting service answer and the first speaks an older protocol version
- **WHEN** voice control connects
- **THEN** it is served by the second copy and a spoken command is carried out
