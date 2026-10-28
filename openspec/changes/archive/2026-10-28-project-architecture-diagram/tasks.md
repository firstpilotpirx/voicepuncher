## 1. Crate check (Red, then Green)

- [x] 1.1 Red: add `Rule: The architecture page shows every workspace crate` to `dev-tools/tests/features/change_workflow.feature` with its three scenarios (a member missing from the page, an extra crate on the page, the real page matches the workspace) and their steps in `dev-tools/tests/steps/architecture_page/`; run `just test-one 'scenario: <name>'` and see them fail
- [x] 1.2 Green: `crate_differences(page, members)` in the steps returns the missing and the extra crates read from the `Workspace crates` c4 block; the two synthetic scenarios pass; commit

## 2. Architecture page

- [x] 2.1 `kit report new docs/architecture/project-architecture.html`; write the seven pictures of the design (crates, layers, primary adapters, two secondary adapter pictures, runtime, one dictation), the legend, the ports table and the note on what the test checks; `kit report build` and `kit report shot`, fix every overlap and cut label
- [x] 2.2 The real-page scenario passes; `just test`; commit

## 3. Links and stale overview

- [x] 3.1 Link the page from README (`## Architecture`), the `docs/` row of the `AGENTS.md` repository layout, `docs/architecture/overview.md` and `docs/project-map.md`; `overview.md` lists all ten workspace members
- [x] 3.2 `just test`; commit
- [x] 3.3 No version bump: documentation and a test only, no behavior of the app changes
