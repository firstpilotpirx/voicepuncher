## Why

Two tests in `app/tests/textpasting_macos_it.rs` — `should_bring_back_copied_text_with_every_type_after_the_paste` and `should_not_overwrite_a_copy_made_after_the_paste` — fail when several checkouts run `just test` or `just full-check` at the same time. The macOS pasteboard is one per machine, but their nextest test group serializes them only within one nextest process; another checkout's run writes the pasteboard in between. Seen three times on 2026-10-20/21 on `feat/file-logging` while five other checkouts ran `just full-check`: the restored clipboard held another run's text ("copied after the paste") or no items at all. Each passed on one unchanged rerun, so they are flakes, and the rule says a repeated flake gets a change for its cause.

## What Changes

- The pasteboard tests hold a machine-wide lock while they touch the pasteboard, so runs from parallel checkouts take turns instead of overwriting each other's clipboard.

Current stage: PoC. Target stage: PoC.
Scope: the tests that read or write the system pasteboard, and the lock they take.
Acceptance bar: the pasteboard tests pass on every run while other checkouts run `just test` or `just full-check` beside them; `just test` stays without the model machine lock.
Out of scope: the paste behavior of the app itself; other flaky tests.

## Capabilities

### Modified Capabilities

- none: test infrastructure only.

## Impact

- `app/tests/textpasting_macos_it.rs`, possibly `.config/nextest.toml` and a small lock helper shared by the tests.

## Open points

- Which lock: settled in design D1 — a dedicated pasteboard file lock under `~/.cache/voicepuncher/`, so `just test` never waits for a model run.
- Whether other tests touch the pasteboard: settled in design Context and D4 — only the manual `@real_keyboard` paste step, which takes the same lock.

## Release

SemVer: no release — tests only, no behavior change.
Target version: unchanged.
