# change-workflow Specification

## Purpose
How a change branch lands on main: through the landing queue, in groups checked once.

## Requirements

### Requirement: Branches land in groups through the landing queue

A change branch that changed a build input SHALL join the landing queue after its own `just check` passed, instead of being refused for being behind `main`. The queue leader SHALL merge up to four queued branches onto `main`, run one `just full-check` on the combined state and land every member of a passing group; a failing group SHALL be split until the failing branch is found, and the other members SHALL land.

#### Scenario: A checked branch behind main joins the queue

- **GIVEN** a branch whose own `just check` passed
- **AND** `main` moved after that check
- **WHEN** the session lands the branch
- **THEN** the branch joins the landing queue instead of being sent to check again

#### Scenario: A group lands after one full check

- **GIVEN** three queued branches, each with a passing `just check`
- **WHEN** the queue leader lands the group
- **THEN** one `just full-check` runs on the three branches merged onto `main`
- **AND** all three land on `main` once it passes

#### Scenario: A red group lands its green members

- **GIVEN** a queued group in which one branch breaks the full check
- **WHEN** the group's full check fails
- **THEN** the group is split until the breaking branch is found
- **AND** the other branches land
- **AND** the breaking branch is sent back to its session with the failure

#### Scenario: A branch without build inputs needs no full check

- **GIVEN** a branch that changed only docs or OpenSpec documents
- **WHEN** the session lands it
- **THEN** it lands after `just test` without waiting for a group's full check
