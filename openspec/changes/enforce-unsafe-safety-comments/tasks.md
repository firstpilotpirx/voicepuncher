## 1. Thread-crossing allowlist

- [ ] 1.1 Red: `app/tests/unsafe_impls.rs` fixture tests — an unlisted `unsafe impl Send` fails naming type and file; a listed one with `// SAFETY:` above passes; a listed one without it fails; a stale entry fails
- [ ] 1.2 Green: the scan, the allowlist with file and reason, and the workspace test over every member's sources
- [ ] 1.3 `just test`; commit

## 2. SAFETY comments and the lints

- [ ] 2.1 Deny `undocumented_unsafe_blocks` and `multiple_unsafe_ops_per_block` in `[workspace.lints.clippy]`
- [ ] 2.2 Write a `// SAFETY:` comment for every site clippy reports, after reading it; split every block with several operations
- [ ] 2.3 `cargo fmt --all`, `just lint` clean, `just test`; commit

## 3. Rules text and version

- [ ] 3.1 `AGENTS.md` § Architecture rules: every `unsafe` carries a `SAFETY:` comment, one operation per block, the thread-crossing allowlist
- [ ] 3.2 Keep `version` at 0.36.1 (no release: comments and checks only); manual checks: none
- [ ] 3.3 `just test`; commit
