# Tasks

No Rust code and no Gherkin scenario: the lock and the stage order are proved by recipe self-tests the `test` stage runs (design.md § Test levels).

## 1. One lock phase (decisions 1, 2, 7)

- [ ] 1.1 Red: `_test-e2e-locks` (replacing `_test-class-locks`) expects `_e2e-lock` to make two model phases take turns, to be kept out by an exclusive machine-lock run, to wait for a running quiet build and keep a later one out, quiet builds to overlap, and the lock wait to default to 6 h; `_test-stage-order` (replacing `_test-lock-phases`) checks the stage lists; run them, they fail (no `_e2e-lock`).
- [ ] 1.2 Green: `_e2e-lock` (machine lock exclusively, then the quiet lock exclusively); `_stages` runs every model stage to run in one `e2e` phase under it; drop `_class-lock`, `_lock-phases`, `light_stages`, `exclusive_stages`; the `test` stage runs the new self-tests; run them, they pass; commit.

## 2. Ports and report (decisions 3, 4)

- [ ] 2.1 Drop the light ports and `VOICEPUNCHER_TEST_LIGHT_*`; `heavy_ports` → `test_ports`; the stage table and journal show the `e2e` lock row; `just test`; commit.

## 3. Docs (decisions 5, 6)

- [ ] 3.1 `AGENTS.md` (§ Several changes at once, § How to verify a change: Lock classes, Local services, Fail fast, the `full-check` row) and the `justfile` comments describe one exclusive model phase and standalone `just test` waiting; commit.
- [ ] 3.2 Version: no bump — test tooling only (proposal § Release); manual checks: none.
