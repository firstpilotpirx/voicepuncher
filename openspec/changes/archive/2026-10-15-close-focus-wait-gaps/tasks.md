## 1. Hold the text back in web content without a text input

- [x] 1.1 Red: in `app/src/core/domain/text_focus.rs` add unit tests: wait on `AXWebArea` and `AXGroup` in web content with no settable value, also with a text selection; paste on every text role in web content; paste on a settable `AXGroup` in web content; paste on `AXWindow`, `AXGroup`, `AXScrollArea`, `AXUnknown` outside web content; the description appends ` +web`. Add `in_web_content` to `FocusedElement` (false at every existing construction) so they compile; run `just test` and see them fail
- [x] 1.2 Red: in `app/tests/features/slice/text_insertion.feature` add the scenarios "Paste at once into a text input on a web page", "Paste at once into an editable element on a web page", "Hold the text back when a web page has focus without a text input", "Hold the text back when text is selected on a web page", "Paste at once into an element of a generic kind" (now a terminal's window or group outside web content) and "Paste once the user clicks into a text input on the web page", with their steps in `app/tests/steps/text_insertion.rs`; in `app/src/core/application/text_insertion.rs` add a unit test for the decision line of a web page; run `just test` and see the hold-back scenarios fail
- [x] 1.3 Commit the failing tests on their own: `test: reproduce paste into web content without a text input` (intentionally red)
- [x] 1.4 Green: implement the web content rule in `TextFocus::of` and the ` +web` description; `just test` green
- [x] 1.5 Green: `MacOsFocusProbe` sets `AXManualAccessibility` on the focused app before reading the focus and fills `in_web_content` by walking `AXParent` up to an `AXWebArea` (at most 64 steps); extend `app/tests/focus_macos_it.rs` so the gated test logs the web content flag; `just test` green
- [x] 1.6 Refactor; update `docs/ubiquitous-language.md` § Text input focus; `just check`; commit `fix: hold the text back in web content without a text input`

## 2. Release

- [x] 2.1 Bump `version` in `app/Cargo.toml` to the next PATCH on top of `main` (0.13.2 after the merge of main); `just check`; commit
- [x] 2.2 Manual checks by the user (README § Release checklist): with `cargo run --release -p voicepuncher` and `just start-model-servers` running — in Chrome and in an Electron app (Slack or VS Code) dictate with no input focused: the overlay shows the focus wait and the log has `+web → wait`; click into an input: the text is pasted; dictate with an input focused: pasted at once; dictate into a terminal: pasted at once
