# Tasks

## 1. Headphones and sparkles as stage icons

- [x] 1.1 Red: add `should_show_headphones_for_the_speech_recognition_stage` to `mod tests` in `app/src/adapters/secondary/visualfeedback/macos/mod.rs`, asserting `stage_icon(OverlayState::RecognizingSpeech) == Some("\u{1F3A7}")` (design D2); run `just test` and verify it fails on 👂
- [x] 1.2 Red: add `should_show_sparkles_for_the_text_refinement_stage` beside it, asserting `stage_icon(OverlayState::RefiningText) == Some("\u{2728}")`; run `just test` and verify it fails on ✏️
- [x] 1.3 Green: in `stage_icon`, return 🎧 (U+1F3A7) for `RecognizingSpeech` and ✨ (U+2728) for `RefiningText`, without U+FE0F (design D1); verify `just test` is green, including the unchanged `should_give_each_processing_stage_an_icon_of_its_own`
- [x] 1.4 Change the doc comment on `apply_processing_state` from "beside it" to "to its right"; verify `grep -rn -e '👂' -e '✏' app docs README.md` finds nothing
- [ ] 1.5 Run `just check`, verify it passes, and commit (`fix(overlay): show headphones and sparkles as stage icons`)

## 2. Release

- [x] 2.1 Bump `version` in `app/Cargo.toml` from 0.14.1 to 0.14.2 (or the next patch above `main`, `kit version-check`), update `Cargo.lock` with `cargo update -p voicepuncher --offline`; verify `just test` passes
- [ ] 2.2 Run `just check`, verify it passes, and commit (`chore(release): bump version to 0.14.2`)
- Manual checks by the user (stage 10, design D4): `just start-model-servers`, then `cargo run --release -p voicepuncher` once in light and once in dark appearance with text refinement on — 🎧 with the spinner turning to its right, then ✨ the same way, both in color, whole and legible, overlay hides at the paste; once with `--no-llm` — only 🎧; `◉ REC`, `⌨ FOCUS` and the `stop→paste` time as before; the user pastes the log.

## Workflow follow-up

- `/opsx:verify` and the `spec-test-reviewer` subagent, then `/opsx:archive`.
- `just full-check` before the merge (`app/` is a build input).
- `just release` after the merge.
