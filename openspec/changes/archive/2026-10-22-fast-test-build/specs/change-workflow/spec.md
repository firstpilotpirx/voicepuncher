# Spec Delta

## ADDED Requirements

### Requirement: Integration tests build into one binary per member

The integration tests of a workspace member SHALL build into one test binary, besides the Gherkin harnesses and the tests that must run on the main thread, so that an edit relinks one binary per member instead of one per test file. The checks SHALL fail on a test file under that binary's directory that the binary does not include, naming the file. Every test SHALL keep its own process, its time limit, its test group and its place in the run order.

#### Scenario: A one-file edit relinks one binary per member

- **GIVEN** a workspace whose test targets are built
- **WHEN** one source file of the application is edited and the test targets are built again
- **THEN** the application's integration tests are linked once, as one binary
- **AND** the Gherkin harnesses and the main-thread tests are linked as their own binaries

#### Scenario: A test file the binary leaves out fails the checks

- **GIVEN** a new test file under a member's integration test directory
- **AND** the member's integration test binary does not declare it
- **WHEN** the checks run
- **THEN** they fail and name the file

#### Scenario: A moved test keeps its isolation and limits

- **GIVEN** a test that drove a shared service one at a time before the move
- **WHEN** the test suite runs after the move
- **THEN** that test still runs one at a time, in a process of its own, within its time limit
- **AND** the suite runs the same tests as before the move
