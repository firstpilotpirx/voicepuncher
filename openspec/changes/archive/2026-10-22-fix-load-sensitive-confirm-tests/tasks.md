# Tasks

## 1. A copy-port window of its own for each two-copies test

- [x] 1.1 Red: add `should_give_each_two_copies_test_a_window_of_copy_ports_of_its_own` to `command-spotting/src/confirm/tests.rs` (two `two_copies` calls, listeners held: disjoint copy ports, all below the ephemeral range), run it, see it fail on the overlapping windows
- [x] 1.2 Green: `two_copies` picks its base from `30_000 … 34_999` in steps of 8 from the slot the process id gives and scans on past a bind that fails; the new test and the two-copies tests pass
- [x] 1.3 Loop the three confirmation tests 20 times under 20 busy processes and beside `just e2e-resilience`; record the counts, the silent-copies test's included, in `design.md` § Results
- [x] 1.4 `just test`; commit

## 2. Version and manual checks

- [x] 2.1 Version unchanged (test-only change, no release); manual checks: none
