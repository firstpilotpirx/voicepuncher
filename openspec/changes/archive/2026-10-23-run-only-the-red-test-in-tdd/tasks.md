# Tasks

## 1. Shared named-run machinery and `test-one`

- [x] 1.1 Red: add a `justfile` self-test of the name-to-selection mapping (bare name, exact name, `scenario:` name) for `test-one`; run it, see it fail
- [x] 1.2 Green: move the name-to-selection step of `rerun-failed` into a private recipe that also takes bare names; self-test passes
- [x] 1.3 Red: add a self-test of the never-ran check on a captured log (bare name seen / not seen, scenario seen / not seen); run it, see it fail
- [x] 1.4 Green: move the run-and-check step of `rerun-failed` into a private recipe with the bare-name rule; self-test passes
- [x] 1.5 Red: add a self-test that `test-one` runs the `test` stage under its own journal name with the rerun marker; run it, see it fail
- [x] 1.6 Green: add the `test-one` recipe on the shared recipes; rewrite `rerun-failed` on them; self-test passes
- [x] 1.7 Run `just test-one` on one real unit test and one slice scenario and a name that matches nothing; see the results the specs name
- [x] 1.8 `just test` and commit

## 2. Workflow documents

- [x] 2.1 `AGENTS.md` § TDD and § How to verify a change: Red and Green run `just test-one`; `just test` once per task group; verify `kit` doc checks pass in `just test`
- [x] 2.2 agent-kit `tdd` skill: Red runs only the new test through the project's one-test command; land it as an agent-kit PR
- [x] 2.3 `just test` and commit

## 3. Release and manual checks

- [x] 3.1 No version bump: no release (proposal § Release); `just check` once
- [x] 3.2 Manual checks by the user: none
