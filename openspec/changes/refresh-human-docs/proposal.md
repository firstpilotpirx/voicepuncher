## Why

The documents agents read stay accurate and the documents only people read have rotted (audit of 2026-10-22): `docs/architecture/overview.md:40, 108` describes text refinement as `llama-cpp-2` FFI with Qwen2.5-3B (the adapter is `textrefinement/openai` over HTTP to `llama-server` with Qwen3-4B); `overview.md:111` says sounds play through `NSSound`/`AVAudioPlayer` (the code runs `afplay`); `docs/architecture/context.md:46` says the GGUF is loaded into RAM at startup; `docs/project-map.md:13` names a `release-check` recipe that does not exist; `CHANGELOG.md` stops at 0.3.1 (2026-10-04), 55 releases ago; `AGENTS.md` line 17 lists the workspace members without `copy-pool`. `docs/project-map.md`, the specs and the glossary are accurate.

## What Changes

- `overview.md` and `context.md` are rewritten from the code as it is (the four services, the copy pool, the command spotting protocol, the runtime and the overlay) and cut to what `project-map.md` and the specs do not already say; the C4 diagrams match the module tree.
- `CHANGELOG.md` is either generated from the GitHub Releases (one entry per version, the changes named) by a `just changelog` recipe run at release, or replaced by a pointer to the Releases page — decided in design; it never again lags by hand.
- `AGENTS.md` names `copy-pool`; every claim in `AGENTS.md`, `README.md` and `docs/` that names a recipe, a file or a port is checked by a test (`app/tests/docs_it.rs` or a `kit` command) that fails when the named thing does not exist.

Current stage: PoC. Target stage: PoC.
Scope: `docs/architecture/*.md`, `docs/project-map.md`, `CHANGELOG.md`, `AGENTS.md` line 17, `README.md`, the docs check.
Acceptance bar: the docs check passes; a reader of `overview.md` finds every service and crate the app has today and none it no longer has.
Out of scope: the specs and the glossary (accurate); release notes of batch landing (an agent-kit change).

## Capabilities

### Modified Capabilities

- none (documents only).

## Impact

- `docs/**`, `CHANGELOG.md`, `AGENTS.md`, `README.md`, `justfile` (`changelog`), a docs test.

## Open points

- Generate `CHANGELOG.md` or drop it; proposed: generate at `just release` from the Releases, so one source exists.

## Release

SemVer: no release.
Target version: unchanged.
