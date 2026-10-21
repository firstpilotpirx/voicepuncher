# Tasks

No Rust code and no spec scenario (`skip_specs: true`): keys are tested by the recipe self-test `_test-stage-keys`, selection and summary by recipe runs at scratch `VOICEPUNCHER_STAGE_PASSES` / `VOICEPUNCHER_STAGE_JOURNAL` (design.md § Test levels).

## 1. Part keys (decision 1)

- [x] 1.1 Red: `_test-stage-keys` makes its scratch repository a four-member workspace and expects `test@<dir>` keys to follow package scope (a `command-spotting/` file changes `test@command-spotting` and `test@e2e-test`, not `test@app` or `test@copy-pool`; a `copy-pool/` file changes every part; a file outside the members changes every part; a `justfile` edit changes `test@justfile`, a `docs/` edit no part); run it, it fails.
- [x] 1.2 Green: `_stage-keys` parses members and path dependencies and hashes `test@<dir>`, `test@justfile`, `test@members`; `_test-stage-keys` passes; `just test`; commit.

## 2. Selection in `_test` (decision 2)

- [x] 2.1 `_record-pass <stage> <key> <seconds>` recipe; `_stages` writes its pass records through it.
- [x] 2.2 `_test` reuses parts with a record, runs the rest with `-p`, runs the self-tests only when `test@justfile` is not reused, records the parts that passed and whose key held; rerun keeps the old selection. Check: `just _test` twice with scratch passes runs no package the second time; `VOICEPUNCHER_REUSE_STAGES=0` runs all; `just test`; commit.

## 3. Journal and summary (decision 3)

- [x] 3.1 `_stage-journal-append` copies the pass record's `seconds` into `from`; `stage-times` shows a `saved` column and a `reuse saved` total. Check on a scratch journal; `just test`; commit.

## 4. Docs

- [x] 4.1 `AGENTS.md` § How to verify a change (passed stages are reused: the `test` stage per package); `justfile` comments; commit.
- [x] 4.2 Version: no bump — test tooling only; manual checks: none.
