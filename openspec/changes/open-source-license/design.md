## Context

The project goes public. A license is a stack choice made once per project; the owner's choice for every public project is `MIT OR Apache-2.0`.

## Decisions

| Question | Chosen | Why | Rejected |
| --- | --- | --- | --- |
| License | `MIT OR Apache-2.0` | the owner's choice; the Rust norm; Apache adds a patent grant, MIT keeps GPLv2 compatibility | MIT alone: no patent grant; GPL-3.0: copyleft the owner does not want |
| Copyright holder | `firstpilotpirx` | the public identity of every commit of this repository | the owner's legal name: the repository carries none |
| Text layout | `LICENSE-MIT` and `LICENSE-APACHE` at the root, official texts | the form rust-lang, serde and tokio use; tools detect both | one `LICENSE` naming both: tools miss the second |
| Manifest field | `license` in `[workspace.package]`, `license.workspace = true` in each member | one place to change | the field in every member |
| Texts in the bundle | copied by `just bundle` into `Contents/Resources` next to `licenses/` | the disk image is the distributed form; MIT and Apache require the notice to travel with copies | only in the repository |

## Test levels

| Scenario | Level |
|---|---|
| The bundle holds the application's own license | E2E (`@bundle`, the `e2e-bundle` stage) |
