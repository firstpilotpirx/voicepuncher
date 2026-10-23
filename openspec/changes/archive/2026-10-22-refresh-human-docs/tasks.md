# Tasks

## 1. The docs check

- [x] 1.1 Red: `app/tests/it/docs_check/` with tests over fixture documents — a stale recipe, a stale path, an unknown port each fail naming document, line and thing; a clean fixture passes — run them and see them fail
- [x] 1.2 Green: the checker in `app/tests/it/docs_check/` (recipes from the `justfile` headers, rooted paths on disk or git-ignored, ports after `port` or `:` against the declared set); the fixture tests pass
- [x] 1.3 Red: the test over `AGENTS.md`, `README.md` and `docs/**/*.md` runs and fails on the stale names (the bench ports 8090 and 8091 in `README.md`, the eight files of `docs/e2e-benchmark-design.md`); `just test`, commit

## 2. The documents name only what exists

- [x] 2.1 `CHANGELOG.md` becomes the pointer to the GitHub Releases page; `docs/project-map.md` drops `release-check`
- [x] 2.2 `docs/e2e-benchmark-design.md` gets its implemented-design note and today's file names
- [x] 2.3 `AGENTS.md` line 5 says a workspace of four crates and the layout tree lists `copy-pool/`; the test of 1.3 passes; `just test`, commit

## 3. The architecture documents match the code

- [x] 3.1 `docs/architecture/overview.md` rewritten: component diagram with one node per adapter directory, pipeline state machine, corrected technology table (HTTP refinement, `afplay`, command spotting, copy pool, overlay), quality goals with today's models; no module table
- [x] 3.2 `docs/architecture/context.md` rewritten: context diagram with the four services and the model files, external systems table (the services, `afplay`, no in-process model), permission flow
- [x] 3.3 `just test` (the docs check over the rewritten documents passes), commit; `just check` once

## 4. Version and manual checks

- [x] 4.1 `version` in `app/Cargo.toml` stays (no release); manual checks: none
