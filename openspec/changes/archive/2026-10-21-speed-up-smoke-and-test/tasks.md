# Tasks

No spec scenario (`skip_specs: true`): test tooling only (design.md § Test levels).

## 1. One nextest test per rule (decision 1)

- [x] 1.1 Red: unit tests in `e2e-test/src/harness_args.rs` for listing one test per rule, a slug name, scenarios outside rules under the file's name, and running only a named rule; run them, they fail.
- [x] 1.2 Green: `harness_args` rule listing and selection; `app/tests/cucumber_integration.rs` runs only the named rule's scenarios; `just test`; commit.

## 2. Shorter waits (decision 3)

- [x] 2.1 `commandspotting_client_it`: late stub 6 s; `just test`.
- [x] 2.2 `commandspotting_websocket_it`: sentences and commands of one test in parallel connections.
- [x] 2.3 Shorten other waits of the slowest `just test` tests where they outlast more than they must; `just test`; commit.

## 3. Smoke parts in one nextest call (decision 2)

- [x] 3.1 Red: `_test-lock-phases` expects each smoke part to select its binaries and the harness in one nextest call; run it, it fails.
- [x] 3.2 Green: `_e2e-smoke-part` builds one filter for its binaries and `cucumber_e2e`, intersected with a rerun's filter; `.config/nextest.toml` and `justfile` comments; `just test`; commit.

## 4. Docs and version

- [x] 4.1 `AGENTS.md` and `justfile` comments where they describe the per-file harness tests or the smoke part order; commit.
- [x] 4.2 Version: no bump — tests and recipes only (proposal § Release); manual checks: none.
