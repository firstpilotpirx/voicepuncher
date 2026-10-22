## ADDED Requirements

### Requirement: Every unsafe site states why it is sound

The project's checks SHALL fail on an `unsafe` block or `unsafe impl` without a `// SAFETY:` comment directly above it, and on an `unsafe` block holding more than one unsafe operation. They SHALL fail on an `unsafe impl Send` or `unsafe impl Sync` that the allowlist of thread-crossing types does not list with its file and its reason, and on an allowlist entry that no longer matches an `unsafe impl` in the code.

#### Scenario: An unsafe block without a SAFETY comment fails the checks

- **GIVEN** an `unsafe` block with no `// SAFETY:` comment above it
- **WHEN** the checks run
- **THEN** they fail and name the block's file and line

#### Scenario: An unsafe block with two unsafe operations fails the checks

- **GIVEN** an `unsafe` block that calls two unsafe functions
- **WHEN** the checks run
- **THEN** they fail and name the block's file and line

#### Scenario: A new unsafe impl Send outside the allowlist fails the checks

- **GIVEN** an `unsafe impl Send` for a type the allowlist does not list
- **WHEN** the checks run
- **THEN** they fail and name the type and its file

#### Scenario: A listed unsafe impl Send keeps passing

- **GIVEN** an `unsafe impl Send` for a type the allowlist lists with its file and reason
- **AND** a `// SAFETY:` comment above it
- **WHEN** the checks run
- **THEN** they pass

#### Scenario: A stale allowlist entry fails the checks

- **GIVEN** an allowlist entry whose type has no `unsafe impl Send` or `Sync` left in its file
- **WHEN** the checks run
- **THEN** they fail and ask for the entry to be removed
