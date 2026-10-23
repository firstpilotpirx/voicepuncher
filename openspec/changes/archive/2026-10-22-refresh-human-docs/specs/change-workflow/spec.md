# Spec Delta

## ADDED Requirements

### Requirement: Documents name only things that exist

The project's checks SHALL fail when a document written for people or agents (`AGENTS.md`, `README.md`, `docs/**/*.md`) names a `just` recipe, a repository path or a loopback port that does not exist in the repository, naming the document, the line and the missing thing; a document whose every named recipe, path and port exists SHALL pass.

#### Scenario: A document names a recipe that does not exist

- **GIVEN** a document names a task-runner recipe the task runner does not list
- **WHEN** the project's checks run
- **THEN** they fail, naming the document, the line and the recipe

#### Scenario: A document names a path that does not exist

- **GIVEN** a document names a repository file or folder that is not in the repository
- **WHEN** the project's checks run
- **THEN** they fail, naming the document, the line and the path

#### Scenario: A document names a port the code does not use

- **GIVEN** a document names a loopback port that no service default in the code uses
- **WHEN** the project's checks run
- **THEN** they fail, naming the document, the line and the port

#### Scenario: Every named thing exists

- **GIVEN** every recipe, path and port the documents name exists
- **WHEN** the project's checks run
- **THEN** the documents check passes
