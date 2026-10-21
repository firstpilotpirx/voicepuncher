# Design

No architectural decision.

## Decisions

1. **The `justfile` exports `MACHINE_LOCK_WAIT`, default 21600 s.** The kit's own default may differ between kit versions (it was 3600); exporting it from the `justfile` makes every locked recipe, and the class lock's outer deadline in `_class-lock` (wait + deadline), use 6 h whatever kit is installed. A caller that sets `MACHINE_LOCK_WAIT` keeps its value.
2. **A failed test is never rerun unchanged.** `AGENTS.md` drops the "rerun once" allowance from § Flakes and from § After a failure: `just rerun-failed` runs only after a commit that tries a fix; a test that failed for a cause outside the change is filed with `/create-change` (test, recipe, what ran beside it) and the branch declares that change in `depends_on`. `VOICEPUNCHER_REUSE_STAGES=0` is no longer a way to rerun a suspected flake.

## Test levels

| Behavior | Level |
|---|---|
| `MACHINE_LOCK_WAIT` defaults to 21600 and keeps a caller's value | recipe self-test `_test-class-locks`, run by the `test` stage |
| Flake policy wording | review of `AGENTS.md` (acceptance bar: `grep rerun AGENTS.md`) |
| A queued `full-check` waits past 60 min and runs | journal after the merge (`lock_wait` > 3600 with stages run); not run in this change (batch mode) |
