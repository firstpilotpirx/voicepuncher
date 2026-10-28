## ADDED Requirements

### Requirement: The architecture page shows every workspace crate

The project SHALL keep one offline architecture page, `docs/architecture/project-architecture.html`, whose crate diagram names exactly the members of the Cargo workspace; the project's checks SHALL fail on a member missing from the diagram or a diagram crate that is not a member, naming the crate.

#### Scenario: A workspace crate is missing from the page

- **GIVEN** the Cargo workspace has a member the page's crate diagram does not name
- **WHEN** the project's checks run
- **THEN** they fail, naming the missing crate

#### Scenario: The page names a crate that is not in the workspace

- **GIVEN** the page's crate diagram names a crate that is not a workspace member
- **WHEN** the project's checks run
- **THEN** they fail, naming the extra crate

#### Scenario: The page matches the workspace

- **GIVEN** the page's crate diagram names exactly the workspace members
- **WHEN** the project's checks run
- **THEN** the architecture page check passes
