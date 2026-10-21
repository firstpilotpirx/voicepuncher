## ADDED Requirements

### Requirement: Code stays within size and complexity budgets

The project's checks SHALL fail on a unit of Rust code over a size or complexity budget (function 40 code lines, 5 parameters, nesting 4, cognitive complexity 15, file 300 code lines, struct 12 fields, type 35 public members, module directory 2 000 code lines) unless the size baseline lists it. A listed unit SHALL fail them when it grows past its listed measure or falls within its budget while still listed.

#### Scenario: A new function over budget fails the checks

- **GIVEN** a function of 41 code lines that the size baseline does not list
- **WHEN** the checks run
- **THEN** they fail and name the function and its length

#### Scenario: A listed unit that grew fails the checks

- **GIVEN** a function the size baseline lists with 52 code lines
- **AND** the function now has 53 code lines
- **WHEN** the checks run
- **THEN** they fail and name the function, its listed and its current length

#### Scenario: A listed unit that shrank keeps passing

- **GIVEN** a function the size baseline lists with 52 code lines
- **AND** the function now has 45 code lines
- **WHEN** the checks run
- **THEN** they pass

#### Scenario: A listed unit back within budget must leave the baseline

- **GIVEN** a file the size baseline lists with 420 code lines
- **AND** the file now has 280 code lines
- **WHEN** the checks run
- **THEN** they fail and ask for the file's entry to be removed from the baseline

#### Scenario: A struct with too many fields fails the checks

- **GIVEN** a struct with 13 fields that the size baseline does not list
- **WHEN** the checks run
- **THEN** they fail and name the struct and its field count

#### Scenario: A type with too large a public surface fails the checks

- **GIVEN** a type with 36 public methods that the size baseline does not list
- **WHEN** the checks run
- **THEN** they fail and name the type and its public member count

#### Scenario: A module directory over budget fails the checks

- **GIVEN** a module directory of 2 001 code lines that the size baseline does not list
- **WHEN** the checks run
- **THEN** they fail and name the directory and its size

#### Scenario: Comments and blank lines do not count

- **GIVEN** a function of 30 code lines with 20 comment and blank lines between them
- **WHEN** the checks run
- **THEN** the function counts as 30 code lines and passes
