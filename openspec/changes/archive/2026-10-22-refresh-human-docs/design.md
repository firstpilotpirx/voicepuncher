# Design

## Context

See proposal.md § Why. Nothing fails when a document names a recipe, file or port that is gone. The hosted CI runs `just test` without the kit, so the check must be a workspace test.

## Goals / Non-Goals

**Goals:** the people-read documents describe the code of today; a stale recipe, path or port fails `just test` on the next change.

**Non-Goals:** prose claims, the specs, the glossary, generated documentation.

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| `CHANGELOG.md` | A short pointer to the GitHub Releases page, which `just release` fills from the merged PRs | One source, nothing to run at release | A `just changelog` recipe: a second copy of the notes and a step in the release path |
| Where the docs check lives | A plain Rust integration test, `app/tests/it/docs_check/`, run by `just test` and the hosted CI | CI has no kit; a test fails the PR like any other | A `kit` command (not on CI); a recipe (not in `just test`) |
| A recipe name | A backticked `just <name>`; the names come from the `justfile` recipe headers, not a `just` subprocess | No `PATH` dependence; prose like "just below" is not backticked | `just --summary` at test time |
| A path | A backticked token rooted at a top-level entry (`app/`, `command-spotting/`, `copy-pool/`, `e2e-test/`, `docs/`, `.config/`, `.github/`, `openspec/`, `justfile`, `Cargo.toml`, …) without glob or placeholder characters; it exists when on disk or git-ignored (`dist/`, the corpus) | Relative names and bundle-internal paths are not repository paths | Every backticked token with a slash (70 false positives) |
| A port | A number in 1024–65535 after the word `port(s)` on its line or after `:` in an address; allowed are the numbers the `justfile`, the non-test Rust sources and the E2E scripts declare plus the copy ports (10000 above a service default) | Sample rates, timeouts and offsets are not ports | Every 4–5 digit number |
| `docs/architecture/*.md` | Rewritten from the module tree: one node per adapter directory, the four services and the model files in the context diagram, the technology table corrected (refinement over HTTP to `llama-server` with Qwen3-4B, sounds through `afplay`, command spotting, copy pool, overlay); the module table stays only in `project-map.md` | The diagrams exist nowhere else | Deleting them |
| `docs/e2e-benchmark-design.md` (related work: eight named files are gone) | Kept as the original design with a note that it is implemented and today's file names | Its reasoning is still the E2E suite's | Deleting it |
| `AGENTS.md` line 5 and the tree | A workspace of four crates | Line 5 contradicts the tree | — |

## Risks / Trade-offs

- [A stale path without backticks is missed] → the rewrite backticks every path.
- [A document names a port only a test uses] → a legitimate test port gets a `justfile` variable, which the check reads.
- [`voice-commands-under-load` edits `AGENTS.md`; `fast-test-build` moves `app/tests/*_it.rs` into one binary] → parallel work, no dependency: the second to merge moves or merges `docs_check/main.rs` and line 5.

## Test levels

| Spec scenario | Level |
|---|---|
| A document names a recipe that does not exist | plain Rust test in `app/tests/it/docs_check/` over a fixture document and `justfile` text |
| A document names a path that does not exist | plain Rust test over a fixture document |
| A document names a port the code does not use | plain Rust test over a fixture document and an allowed set |
| Every named thing exists | plain Rust test over the repository's documents (`just test`) |
