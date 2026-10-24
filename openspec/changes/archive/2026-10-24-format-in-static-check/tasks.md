## 1. The fix command

- [x] 1.1 Red: unit tests in `dev-tools/src/static_fix.rs` — strict when `CI` is set or the recipe is `full-check`, local otherwise; the changed files read from `cargo fmt -l` and `cargo clippy --fix` output
- [x] 1.2 Green: `dev-tools static-fix fmt|clippy` — local: `fmt -l`; clippy check, on failure `clippy --fix`, `fmt -l`, check again; every changed file printed; strict: today's `--check` and `-D warnings`
- [x] 1.3 Commit: `feat(dev-tools): static-fix command`

## 2. The static recipe

- [x] 2.1 Red: scenarios in `dev-tools/tests/features/change_workflow.feature` — a local static run on a scratch crate with an unformatted file and a machine-fixable clippy warning fixes both, names the file and runs the analyzers; the same run with `CI` set fails on formatting and leaves the file unchanged; the recipe runs `static-fix fmt`, `static-fix clippy`, then the analyzers
- [x] 2.2 Green: `{{dev_tools}} static-fix fmt` and `static-fix clippy` in place of the two strict commands of `_static`; `AGENTS.md` § How to verify a change says what a local `static` fixes
- [x] 2.3 Commit: `feat: format-in-static-check`; no version bump — tooling only, the proposal names no release
