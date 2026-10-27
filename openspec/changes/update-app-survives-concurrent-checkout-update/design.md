# Design

## Context

`update-app` starts with `git switch main && git pull --ff-only origin main` in the main checkout. kit fast-forwards the same checkout (`git merge --ff-only origin/main`, only when it is clean and on `main`) right after it lands or files a change. When kit moves `main` while `git pull` is fetching, `pull` notices the head moved ("fetch updated the current branch head") and tries to carry the working tree from the old commit; the files kit just wrote look untracked to it and it stops with exit 128. kit's side is a single fast-forward and is safe; the fragile step is `pull`.

## Goals / Non-Goals

**Goals:** `update-app` reaches the newest `origin/main` whatever kit does at the same moment; a real blocker stops it with one line of its own.

**Non-Goals:** agent-kit's update of the main checkout, the rebuild, the bundle, `cargo watch`.

## Decisions

**A `dev-tools update-checkout` command replaces `git switch main && git pull`.** The recipe calls it through `dev_tools` and goes on to quit, rebuild and relaunch only when it succeeds. Rust rather than shell because it needs a loop, a deadline and a classification of git's failures, and because `dev-tools` is where the recipes' machinery is tested (`change_workflow.feature`). Rejected: retrying `git pull` in the recipe — still a race, untestable, and git's own messages (with `reset --hard`) would reach the user.

**Fetch, then fast-forward, in a loop until about 30 s pass.** Each round: `git fetch -q origin main` (moves only `origin/main`, never the head or the files, so kit's update cannot collide with it); then read the state and act:

| State | Action |
|---|---|
| `.git/index.lock` exists | another git command works on the checkout: wait and retry |
| on `main`, head equal to `origin/main` | done, success (also when kit got there first) |
| head not an ancestor of `origin/main` | stop: local commits not on the remote |
| tracked files changed, or another branch | wait while it may be kit's update in progress; stop once it persists for 3 s |
| behind | `git merge --ff-only -q origin/main`; a failure is retried |

At the deadline it stops with the last blocker. Stop lines go to stderr as `✗ main checkout not updated: <what blocks> — <what to do>`; git's own stderr is captured and never printed whole, so its `reset --hard` advice never shows. A fetch that keeps failing (offline) stops naming the remote.

**Another branch stops instead of `git switch main`.** The proposal names another branch as a blocker, and kit too leaves a checkout on another branch alone; switching under a running agent could carry its work away. The line says `git switch main`.

**The wait and the settle time are constants** (30 s, 3 s, 0.5 s between rounds); tests use the real values — the blocker scenarios each take about 3 s.

## Test levels

| Scenario | Level |
|---|---|
| Another program updates the checkout during the app update | component `.feature` (`dev-tools/tests/features/change_workflow.feature`): scratch remote and clone, a held `index.lock` released by a thread that then fast-forwards the clone |
| A checkout behind main is moved forward | component `.feature` |
| A checkout already at the newest main is left as it is | component `.feature` |
| Local changes stop the app update with one clear line | component `.feature` |
| Another branch stops the app update with one clear line | component `.feature` |
| Local commits not on the remote stop the app update with one clear line | component `.feature` |

A `justfile_rules` unit test checks that `update-app` calls `update-checkout` and holds no `git pull`. The manual run: the user runs `just update-app` once on the main checkout.

## Risks / Trade-offs

- A blocker kit causes for longer than 3 s (a slow disk) stops the update; running it again then succeeds. Accepted: a real blocker must not keep the user waiting 30 s.
- `update-app` now refuses a checkout on another branch where it used to switch; the stop line says what to run.
