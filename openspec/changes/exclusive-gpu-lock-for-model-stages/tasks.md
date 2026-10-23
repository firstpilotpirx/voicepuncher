# Tasks

No Rust code and no Gherkin scenario: the journal and the summary are proved by a recipe self-test the `test` stage runs (design.md § Test levels).

## 1. Hold start and overlap report (decisions 1–3)

- [ ] 1.1 Red: `_test-stage-times` expects a journal line written by `_stage-journal-append` to carry the hold's `locked` time, `just stage-times` to name an overlapping pair of runs and to print `none` without one; the `test` stage runs it; run it, it fails.
- [ ] 1.2 Green: `_stage-journal-append` writes `locked` per lock entry; `stage-times` prints the `e2e lock overlaps:` section; run the self-test, it passes; `just test`; commit.

## 2. Unfreeze the load flakes (decision 4)

- [ ] 2.1 `just frozen-tests run` on the branch; remove from `.config/frozen-tests.toml` every entry whose test passed; list here every entry that failed alone, with its failure; commit.

## 3. Docs

- [ ] 3.1 `AGENTS.md` § How to verify a change (the stage journal, Lock classes) names the hold start and the overlap report; commit.
- [ ] 3.2 Version: no bump — test tooling only (proposal § Release); manual checks: none.
