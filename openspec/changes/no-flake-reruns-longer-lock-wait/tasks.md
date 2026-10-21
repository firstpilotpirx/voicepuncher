# Tasks

No Rust code and no spec scenario (`skip_specs: true`): the lock wait default is tested by a recipe self-test, the flake policy is documentation (design.md § Test levels).

## 1. Lock wait

- [ ] 1.1 Red: `_test-class-locks` in the `justfile` expects `MACHINE_LOCK_WAIT` to evaluate to 21600 when unset and to a caller's value when set; run it, it fails.
- [ ] 1.2 Green: export `MACHINE_LOCK_WAIT` (default 21600) from the `justfile`; `_class-lock` uses it; `just test`; commit.

## 2. Flake policy

- [ ] 2.1 `AGENTS.md` § How to verify a change (Flakes, After a failure, Passed stages are reused) and § Several changes at once: a failed test is a failure, never rerun unchanged; `rerun-failed` only after a fix; out-of-scope cause filed with `/create-change` and declared in `depends_on`; lock wait 6 h; `just test`; commit.
- [ ] 2.2 Version: no bump — tooling and rules only (proposal § Release); manual checks: none.
