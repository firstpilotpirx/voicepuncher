## ADDED Requirements

### Requirement: The app update survives a concurrent update of the main checkout

`just update-app` SHALL bring the main checkout to the newest remote `main` even while another program updates the same checkout: it SHALL wait, for about 30 seconds at most, while another git command holds the checkout, and SHALL count a checkout already at the newest `main` as updated. When the checkout cannot be updated, it SHALL stop before the rebuild with one line naming the blocker and what to do, never advising to discard work.

#### Scenario: Another program updates the checkout during the app update
- **GIVEN** the remote `main` is ahead of the main checkout
- **AND** another program holds the main checkout and then moves it to the newest `main` itself
- **WHEN** the user runs the app update
- **THEN** the update succeeds and the checkout is at the newest `main`

#### Scenario: A checkout behind main is moved forward
- **GIVEN** the remote `main` is ahead of the main checkout
- **WHEN** the user runs the app update
- **THEN** the update succeeds and the checkout is at the newest `main`

#### Scenario: A checkout already at the newest main is left as it is
- **GIVEN** the main checkout is already at the newest `main`
- **WHEN** the user runs the app update
- **THEN** the update succeeds and the checkout is unchanged

#### Scenario: Local changes stop the app update with one clear line
- **GIVEN** the remote `main` is ahead of the main checkout
- **AND** a tracked file of the main checkout has an uncommitted change
- **WHEN** the user runs the app update
- **THEN** the update fails with one line naming the local changes
- **AND** the output never suggests discarding work

#### Scenario: Another branch stops the app update with one clear line
- **GIVEN** the main checkout is on another branch
- **WHEN** the user runs the app update
- **THEN** the update fails with one line naming that branch

#### Scenario: Local commits not on the remote stop the app update with one clear line
- **GIVEN** the main checkout holds a commit the remote `main` does not have
- **WHEN** the user runs the app update
- **THEN** the update fails with one line naming the local commits
