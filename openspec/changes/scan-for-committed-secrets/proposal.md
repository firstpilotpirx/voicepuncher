## Why

The user wants to make sure nothing sensitive was ever committed by accident to any of their projects: secrets, API keys, tokens, passwords, private keys, signing identities, personal data, recordings of the user's voice or local paths that should stay private.

## What Changes

- Scan every project of the user (working copies under `~/Workspaces/my/` and their GitHub repositories), the whole git history included, not only the current tree.
- Report every finding with repository, file, commit and kind of data.
- For each confirmed leak, propose the fix: rotate the secret, remove it from history, add it to `.gitignore`.
- Add a guard so it does not happen again (a secret scanner in `just lint` or a pre-commit hook).

Current stage: PoC. Target stage: PoC.
Scope: all of the user's repositories and their full history.
Acceptance bar: a report of every finding across all projects, each one resolved or confirmed harmless by the user, and a scanner that blocks new secrets.
Out of scope: the app's behavior.

## Capabilities

### Modified Capabilities

- none: audit and tooling only.

## Impact

- Every repository under `~/Workspaces/my/` and on GitHub.
- `.gitignore`, `just lint` or git hooks.

## Open points

- Which projects count as "all": proposed every repository under `~/Workspaces/my/` plus every GitHub repository of `firstpilotpirx`.
- Which scanner: proposed `gitleaks` (history-aware), pinned.
- History rewrite of a public or shared repository needs the user's yes per repository.
- The audit spans several projects; proposed to file the guard part in agent-kit if it applies to every project.

## Release

SemVer: no release — no behavior change in the app.
Target version: 0.37.2.
