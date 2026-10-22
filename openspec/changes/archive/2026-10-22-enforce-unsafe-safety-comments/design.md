## Decisions

**Clippy carries the comment rule.** `[workspace.lints.clippy]` denies `undocumented_unsafe_blocks` (it covers `unsafe` blocks and `unsafe impl`) and `multiple_unsafe_ops_per_block`, so `just lint` fails on a new site with clippy's own message. On `main` clippy reports 34 blocks and 4 impls without a comment and 18 blocks with several operations: every one gets its comment after reading the site, and each block with several operations is split into one block per operation, each with its comment. The comment is at most 3 lines: the invariant and who upholds it (the main thread, the caller, the type's ownership). No baseline: every site is fixed in this change.

**The thread-crossing allowlist is a test of its own, not a rule in `architecture.rs`.** `architecture.rs` is listed in the size baseline and may only shrink, so the rule goes into a new test target `app/tests/unsafe_impls/`. It scans every workspace `.rs` file for `unsafe impl Send for <Type>` and `unsafe impl Sync for <Type>`, and compares them with an allowlist in the same file: `(file, type, trait, reason)`. It fails on an impl not listed, on an entry with no impl left, and on a listed impl without a `// SAFETY:` line directly above it (clippy already demands a comment; the test demands the `SAFETY:` wording). The allowlist today: `MacosVisualIndicator` (`visualfeedback/macos/mod.rs`, Send), `WindowSet` (Send, Sync), `SignWindow` (Send, Sync), `WindowOnMain` (Send). `harden-overlay-thread-model` removes the overlay's entries.

**Main-thread marker: comments only.** The open point asked whether AppKit calls must carry a `MainThreadMarker`. Adding markers changes signatures across the overlay, which `harden-overlay-thread-model` reworks; this change states the main-thread invariant in the comment and names the marker where one is already in scope. No behavior or signature changes.

**No cargo-geiger here.** `adopt-static-analyzers` found that cargo-geiger fails on this workspace and counts `unsafe` with its own script in the metrics baseline; that count covers the proposal's "unsafe cannot grow unnoticed".

**A `msg_send!` counts as one operation.** The macro expands to two unsafe calls, so clippy counts a block holding one `msg_send!` as two; it cannot be split. Such a statement carries `#[expect(clippy::multiple_unsafe_ops_per_block, reason = "one msg_send!, two unsafe calls inside")]`, which fails once it no longer fires. A `sel!` inside an unsafe block also counts, so selectors and framework constants are read into locals before the block.

**Unsafe helper functions become safe.** The focus probe's helpers were `unsafe fn` called from one block of 9 operations; each helper now takes a live `&CFType`, holds its own one-operation blocks and is safe to call.

**Test code is under the same lints.** The socket helper `bind_without_listening`, copied in three integration tests, is split one call per block, each with its comment.

## Risks

- Splitting blocks touches many lines of overlay files other branches edit: conflicts are mechanical; keep one operation per block and its comment.

## Test levels

| Scenario | Level |
|---|---|
| An unsafe block without a SAFETY comment fails the checks | clippy in `just lint` (`undocumented_unsafe_blocks` denied) |
| An unsafe block with two unsafe operations fails the checks | clippy in `just lint` (`multiple_unsafe_ops_per_block` denied) |
| A new unsafe impl Send outside the allowlist fails the checks | unit (`unsafe_impls` fixture) |
| A listed unsafe impl Send keeps passing | unit (`unsafe_impls` fixture) and the workspace scan |
| A stale allowlist entry fails the checks | unit (`unsafe_impls` fixture) |
