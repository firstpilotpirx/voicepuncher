## 1. The binary and its golden fixtures

- [ ] 1.1 Record, with the former runner, the stage keys of a fixture repository and environment and the journal line of a fixture report, as golden values in the new crate's tests
- [ ] 1.2 Add the workspace member `dev-tools/` (`voicepuncher-dev-tools`, program `dev-tools`) with its subcommand dispatch and the child environment without `cargo run`'s variables
- [ ] 1.3 `just test`; commit

## 2. Keys, passes and the journal

- [ ] 2.1 Red: unit tests for the stage keys (golden keys, the former `_test-stage-keys` cases, the test parts)
- [ ] 2.2 Green: `dev-tools keys`, the stage inputs and the pass records (`_stage-keys`, `_record-pass`, `_pass-line`, `_passed-stages`)
- [ ] 2.3 Red: unit tests for the journal line (golden line, a skipped stage, the lock records) and for `stage-times` (overlaps, turns)
- [ ] 2.4 Green: the journal append under a file lock and `dev-tools stage-times`
- [ ] 2.5 `just test`; commit

## 3. The runner

- [ ] 3.1 Red: unit tests for the lock wrappers (e2e, quiet, none with `VOICEPUNCHER_NO_MACHINE_LOCK=1`, the 6 h wait, the missing lock's install command), the filters and gates (former `_test-gates`, `_test-stage-order` selections) and the accuracy choice; the lock's own tests (former `_test-e2e-locks`) in `needs_machine_lock`
- [ ] 3.2 Green: `dev-tools stages` and `model-stages` (phases, reuse, table, interrupt), the test stage's parts, the filters, gates and `accuracy-stage`
- [ ] 3.3 Red: unit tests for the frozen list (former `_test-frozen-tests`), the named runs and the rerun plan (former `_test-named-run`) and the skipped stage (former `_test-stage-skip`)
- [ ] 3.4 Green: `dev-tools frozen`, `freeze-test`, `named-run`, `rerun-failed`
- [ ] 3.5 `just test`; commit

## 4. Engines and services

- [ ] 4.1 Red: unit tests for the engine lock's parsing and its checks
- [ ] 4.2 Green: `dev-tools engines` (former `_engines`)
- [ ] 4.3 Red: unit tests for the services' command lines, copy ports and the supervisor's restart delay
- [ ] 4.4 Green: `dev-tools services start|stop|e2e-start|e2e-stop` and `dev-tools supervise`; the catalog test reads the services' arguments from the binary's source
- [ ] 4.5 `just test`; commit

## 5. The thin justfile

- [ ] 5.1 Red: unit tests that the `justfile` holds no Perl, fewer than 800 lines and no uncommented `|| true`
- [ ] 5.2 Green: rewrite the `justfile` as thin recipes over `dev-tools`, with `_require-machine-lock`; drop the self-test recipes
- [ ] 5.3 Point the docs check, the needs' messages, the analyzers' configuration and baselines and `.config/nextest.toml` at the new crate
- [ ] 5.4 Document the binary and the lock protocol in `AGENTS.md` § How to verify a change and README
- [ ] 5.5 Compare the stage table and journal line of the former and the new runner on one commit
- [ ] 5.6 No version bump: no release (proposal § Release: tooling only)
- [ ] 5.7 `just test`; `just check` once; commit
