## Context

The structure is described in prose and tables: `docs/project-map.md` (folders and modules), `docs/architecture/overview.md` (a Mermaid flowchart of `app/` and a crate table that still says four crates). The workspace has ten members (`Cargo.toml` `members`): `analyzers`, `app`, `command-spotting`, `copy-pool`, `dev-tools`, `e2e-test`, `entry-folders`, `log-line`, `test-steps`, `voice-commands`. The kit's HTML report rule (`html-reports.md`) governs every HTML document under `docs/`: built with `kit report`, pictures as `<pre class="kit">` YAML blocks drawn to inline SVG, no script, at most 12 boxes per picture.

## Goals / Non-Goals

**Goals:**
- One page a newcomer reads top to bottom: crates, then the layers of `app/`, then the adapters, then the running processes.
- A test that keeps the crate picture equal to the workspace members.

**Non-Goals:**
- Generating the pictures from the code; checking anything on the page beyond the crate list.
- Replacing `docs/project-map.md` or `docs/architecture/overview.md`: they stay the detailed reference.

## Decisions

| Question | Chosen | Why | Rejected |
|---|---|---|---|
| Where the page lives | `docs/architecture/project-architecture.html`, beside `overview.md` | the architecture documents are in that folder | `docs/index.html` (would make the kit treat `docs/` as a page tree) |
| How it is drawn | `kit report new`, then `c4` and `sequence` blocks, built with `kit report build` | the kit rule; offline, no script, source kept in the page | hand SVG, Mermaid (needs a script) |
| Pictures | 1 crates (10 boxes: binary / library / test-only by `kind`, solid links for normal dependencies, dashed for dev-dependencies) · 2 layers of `app/` (primary adapters, `runtime` composition, `core/application` ports, `core/domain`, secondary adapters) · 3 primary adapters (one box per folder: `cli`, `ptt`, `menubar`, `settings`, `permissions`, `modeldownload`, `history`) to the ports they drive · 4 secondary adapters for speech and services (`recording`, `transcription`, `commandspotting`, `localservices`, `modelfetch`, `http`) · 5 secondary adapters for the desktop and files (`textpasting`, `keypressing`, `focus`, `visualfeedback`, `soundplaying`, `archiving`, `logging`, `loginitem`, `permissions`) · 6 runtime: user, menu bar app, the two speech recognition services as three copies each behind the copy pool, the command spotting service, models and logs folders, with ports and protocols (HTTP, WebSocket) · 7 one dictation as a `sequence` (hotkey → recorder → transcriber → copy → paste) | ≤ 12 boxes each, split by layer as the rule asks; secondary adapters (15 folders) need two pictures | one big picture per layer |
| Box text | the glossary terms of `docs/ubiquitous-language.md`; each box's `text` is its one-line role | the glossary is the contract | new wording |
| Ports | a table under picture 2: each trait of `core/application` with its one-line role | 20+ ports overflow a picture | a box per port |
| How the test finds the crates | the `<pre class="kit" data-kind="c4">` block whose `title` is `Workspace crates`; every node's `name` is one crate, written as its workspace folder | the block's source stays in the built page, so the test reads YAML lines, not SVG | parsing the drawn SVG; a separate list in the page |
| Where the test lives | a `Rule:` in `dev-tools/tests/features/change_workflow.feature` with steps in `dev-tools/tests/steps/test_rules.rs`, reusing `members()`; a pure function `crate_differences(page, members)` returns the missing and the extra crates, so the two failing scenarios run on a page text written in the step | the requirement belongs to `change-workflow`, whose harness already checks documents (`Documents name only things that exist`) | an `app` integration test |
| Links | a new `## Architecture` line in README (it has no documentation list yet) and the `docs/` row of the `AGENTS.md` repository layout; `overview.md` and `project-map.md` link to the page | where a newcomer looks first | README only |
| Stale overview | `overview.md` crate table lists all ten members | related work: a document this change touches | leaving it for later |

## Test levels

| Scenario | Level |
|---|---|
| A workspace crate is missing from the page | workflow `.feature` (`dev-tools/tests/features/change_workflow.feature`), on a page text written in the step |
| The page names a crate that is not in the workspace | workflow `.feature`, on a page text written in the step |
| The page matches the workspace | workflow `.feature`, on the real page and `Cargo.toml` |

## Risks / Trade-offs

- [Pictures other than the crates drift from the code (a new adapter folder, a new port)] → the page says at the top that the crate picture is checked and the rest is drawn by hand, with the commit it matches; `docs/project-map.md` stays the detailed list.
- [The kit's YAML block format changes] → the test reads only `name:` lines inside the titled block; a format change fails the test loudly, never silently passes.
- [The test is a build input] → `just check` on the branch and the landing queue's `full-check`; the cost is accepted for a check that cannot drift.
