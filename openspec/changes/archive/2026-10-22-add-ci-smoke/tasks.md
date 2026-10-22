## 1. Recipes without the machine lock

- [x] 1.1 Red: a `_test-lock-phases` self-test asserts that `VOICEPUNCHER_NO_MACHINE_LOCK=1` makes `machine_lock` evaluate to `/usr/bin/env` and that it defaults to the agent-kit lock otherwise
- [x] 1.2 Green: `machine_lock` honours `VOICEPUNCHER_NO_MACHINE_LOCK=1`; `_test` skips `_test-class-locks` with a `⊘` line when it is set
- [x] 1.3 Run `VOICEPUNCHER_NO_MACHINE_LOCK=1 just fmt` and see it run without the lock
- [x] 1.4 `just test` passes; commit

## 2. Hosted workflow

- [x] 2.1 `.github/workflows/ci.yml`: macos-15, PR and `main` triggers, concurrency, caches, `just tools`, `just fmt`, `just lint`, `just test` with `VOICEPUNCHER_NO_MACHINE_LOCK=1`
- [x] 2.2 `typos` and the recipe analyzers accept the new files (`just lint`)
- [x] 2.3 `_test` unsets empty `VOICEPUNCHER_*` variables, so model tests skip on a machine without models; nextest profile `ci` leaves out the tests that need the owner's desktop; caches survive a failed run
- [x] 2.4 Push; prove the red path with a temporary commit holding a misformatted line and a failing unit test (the `fmt` and `test` steps both show red, as each check step runs after a failed one), revert it, and see the clean run green within 10 min; commit

## 3. Documentation

- [x] 3.1 `AGENTS.md` § How to verify a change: CI is the floor every PR has, `full-check` in the landing queue stays the merge gate; `VOICEPUNCHER_NO_MACHINE_LOCK`
- [x] 3.2 `README.md` § Release checklist: the `ci` check required by the branch protection of `main` (owner step)
- [x] 3.3 Keep `version` unchanged (no release: CI tooling only); commit
