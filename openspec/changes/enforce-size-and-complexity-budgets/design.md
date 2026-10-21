## Decisions

**Thresholds: the kit's numbers from the start**, the ~140 places over them today (61 functions over 40 lines, 17 over 5 parameters, 56 blocks nested past 4, 4 over complexity 15) carried by a baseline. 40 code lines, 5 parameters, nesting 4, cognitive complexity 15, 300 code lines per file, 35 public members per type. The kit has no struct or directory budget: 12 fields (a struct past a dozen fields holds several concepts — `Pipeline` has 19) and 2 000 code lines per module directory (about seven files at the file budget, the size of one feature).

**Clippy measures what it can; a test measures the rest and the ratchet.** `clippy.toml` sets the four thresholds and `[workspace.lints.clippy]` denies `too_many_lines`, `too_many_arguments`, `excessive_nesting`, `cognitive_complexity` and `type_complexity`, so `just lint` fails on any new unit over budget with clippy's own message. A listed function carries `#[expect(clippy::<lint>, reason = "size baseline")]`: clippy stays quiet while it is over budget and fails (`unfulfilled_lint_expectations` under `-D warnings`) once it is back within budget with the attribute still there. Clippy cannot report a measure to a test, so a new test target `app/tests/size_budgets.rs` parses every workspace `.rs` file with `syn` (already in `Cargo.lock`) and measures function code lines and parameters, file code lines, struct fields, public members per type and module directory code lines. It fails on a unit over budget that the baseline does not list, on a listed unit that grew, on a listed unit back within budget, and on a size-baseline attribute without a baseline entry or the reverse. Nesting and cognitive complexity are ratcheted by presence only (clippy's attribute), since the test cannot reproduce clippy's measure. A separate target: `architecture.rs` is itself over the file budget.

**One baseline file.** `app/tests/size_baseline.toml` lists every unit over budget with its kind, its identifier (path, plus `Type::function` or the module path for functions and types) and its measure. `VOICEPUNCHER_WRITE_SIZE_BASELINE=1 just test` rewrites it from the current code (`just size-baseline`); a rewrite that adds or raises an entry is reviewed like code.

**A code line is a physical line that is neither blank nor comment-only**, as clippy counts it; doc comments do not count. Function length is the body's lines from the opening brace to the closing one, matching clippy's figure.

**Tests are under the budgets from the start**, as the kit says, in the same baseline.

**`rustfmt.toml` sets `struct_lit_width` and `struct_variant_width` to `max_width`** (100), as `quality.md` requires, so line counts measure complexity, not formatting. The reformat lands as its own commit before the baseline is measured. Branches written before it conflict on formatting only: take either side and run `cargo fmt --all`.

**The workflow rule.** `AGENTS.md` § Architecture rules names the budgets and the baseline; § Workflow says a change that touches a listed unit shrinks it or leaves it as it is, and `just test` proves it.

## Risks

- The reformat touches about 146 files: every branch in flight merges with formatting conflicts once. Mitigation above.
- A merged branch that grew a listed unit fails `just test`: shrink the unit, or rewrite the baseline with the reason in the PR.

## Test levels

| Scenario | Level |
|---|---|
| A new function over budget fails the checks | unit (`size_budgets` fixture) and clippy in `just lint` |
| A listed unit that grew fails the checks | unit (`size_budgets` fixture) |
| A listed unit that shrank keeps passing | unit (`size_budgets` fixture) |
| A listed unit back within budget must leave the baseline | unit (`size_budgets` fixture) |
| A struct with too many fields fails the checks | unit (`size_budgets` fixture) |
| A type with too large a public surface fails the checks | unit (`size_budgets` fixture) |
| A module directory over budget fails the checks | unit (`size_budgets` fixture) |
| Comments and blank lines do not count | unit (`size_budgets` fixture) |
