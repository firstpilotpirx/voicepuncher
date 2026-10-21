## 1. Formatter widths

- [ ] 1.1 Add `rustfmt.toml` with `struct_lit_width` and `struct_variant_width` at 100; run `cargo fmt --all`
- [ ] 1.2 `just test`; commit the reformat on its own

## 2. Size measurement and ratchet (test target)

- [ ] 2.1 Red: `app/tests/size_budgets.rs` fixture tests — code-line count skips comments and blank lines; a new function over 40 lines, a struct over 12 fields, a type over 35 public members and a directory over 2 000 lines are reported; a listed unit that grew fails; one that shrank passes; one back within budget fails
- [ ] 2.2 Green: measurement with `syn` in `app/tests/size_budgets/` (functions, parameters, files, structs, public members, directories) and the comparison against a baseline
- [ ] 2.3 Red: a size-baseline attribute without a baseline entry, and an entry without its attribute, fail
- [ ] 2.4 Green: cross-check of `#[expect(clippy::…, reason = "size baseline")]` attributes against the baseline
- [ ] 2.5 `just test`; commit

## 3. Clippy budgets and the baseline

- [ ] 3.1 Add `clippy.toml` thresholds and `[workspace.lints.clippy]` (deny the five lints); `[lints] workspace = true` in every member
- [ ] 3.2 Put `#[expect(…, reason = "size baseline")]` on every function clippy reports; `just lint` is clean
- [ ] 3.3 Write `app/tests/size_baseline.toml` (`just size-baseline`); the workspace rule in `size_budgets.rs` passes
- [ ] 3.4 Add `clippy.toml`, `rustfmt.toml` and the size-budget files to `stage_inputs`; add the `size-baseline` recipe
- [ ] 3.5 `just test`; commit

## 4. Rules text

- [ ] 4.1 `AGENTS.md` § Architecture rules: the budgets, the baseline, how to refresh it; § Workflow: a change touching a listed unit shrinks it or leaves it as it is
- [ ] 4.2 `just test`; commit

## 5. Version and manual checks

- [ ] 5.1 Keep `version` at 0.35.0 (no release: tooling and rules only)
- [ ] 5.2 Manual checks: none
