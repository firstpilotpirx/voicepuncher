## Why

The audit of 2026-10-22 counted 146 `unsafe` sites in 24 files, almost all objc2/AppKit and FFI, and found about a third without a `SAFETY:` comment (`glass.rs:101, 199–206, 302, 335, 346–355`; `screen_watcher.rs:34, 47–64`; `pill_view.rs:199, 201, 282, 285–291`; `sign_badge.rs:120, 142, 174`; `motion.rs:76–78`; `status_item.rs:178`; `modeldownload/macos/mod.rs:297, 324`; `permissions/macos/mod.rs:267, 411, 429, 470`), so a reviewer cannot tell an audited site from a pasted one. Three `unsafe impl Send`/`Sync` rest on convention: `visualfeedback/macos/mod.rs:534` (`MacosVisualIndicator`, no comment at all), `window_set.rs:136–137` and `pointer_sign.rs:44–45` ("all access is on the main queue", enforced only by a `debug_assert!` in `Drop`). The kit's `enforcement-spec.md` requires every `unsafe` to carry a `SAFETY:` justification and `unsafe` blocks to be gated; the project has no such check.

## What Changes

- Every `unsafe` block, `unsafe fn` call site and `unsafe impl` carries a `// SAFETY:` comment of at most 3 lines stating the invariant and who upholds it; the missing ≈ 50 are written after reading each site.
- clippy `undocumented_unsafe_blocks` and `multiple_unsafe_ops_per_block` as errors in `[workspace.lints]`; a test of its own, `app/tests/unsafe_impls/` (`architecture.rs` may only shrink), requires a `SAFETY:` line above every `unsafe impl Send`/`Sync` and lists the allowed ones in an allowlist with the file and the reason, so a new one fails the test.
- The `unsafe` count in the metrics baseline is `adopt-static-analyzers`' (cargo-geiger fails on this workspace), so unsafe cannot grow unnoticed.
- `AGENTS.md` § Architecture rules states the rule.

Current stage: PoC. Target stage: PoC.
Scope: the comments, the lints, the architecture rule and the allowlist; the thread-model fix that removes the `unsafe impl Send` of the overlay is `harden-overlay-thread-model`.
Acceptance bar: `just lint` fails on an `unsafe` without `SAFETY:`; `just test` fails on an `unsafe impl Send`/`Sync` outside the allowlist; every existing site has a comment a reader can check against the code.
Out of scope: removing `unsafe`; changing any behavior.

## Capabilities

### Modified Capabilities

- `change-workflow`: the unsafe rule as a check.

## Impact

- `app/src/adapters/**` (comments), `app/src/adapters/main_queue.rs`, `Cargo.toml` lints, `app/tests/unsafe_impls/`, `app/tests/*_it.rs`, `AGENTS.md`.

## Open points

- Whether a `SAFETY:` comment that merely restates "called on the main thread" is enough for AppKit getters, or the rule demands the `MainThreadMarker` in scope; decided: comments only; markers belong to `harden-overlay-thread-model` (design).

## Release

SemVer: no release — comments and checks only.
Target version: 0.36.1 (unchanged).
