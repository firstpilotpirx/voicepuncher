## 1. Service command lines and model picking

- [ ] 1.1 Red: add `app/tests/localservices_command_lines_it.rs` comparing the `justfile`'s arguments of each service with the catalog's, plus a doctored-text case that must name the service; run it and see the doctored case fail before the comparison exists
- [ ] 1.2 Green: implement the comparison in the test file; `cargo nextest run -p voicepuncher --test localservices_command_lines_it` passes
- [ ] 1.3 Red: catalog unit test "two KWS model folders, older first by name → the newer is used"; see it fail
- [ ] 1.4 Green: one `newest` helper for the VAD file and the KWS folder in `catalog/mod.rs`; `ls -td` for the KWS folder in the `justfile`; the test passes
- [ ] 1.5 `ServiceProbe::answer` takes a kind and a port; `running_copies` builds no dummy spec; supervision, fakes, `HttpProbe` and tests follow; `just test` passes
- [ ] 1.6 One `copies` method and a `Service` value in the catalog; `spec` without `too_many_arguments`; size baseline entry dropped; `just test` passes and commit

## 2. Shared HTTP agents and AppKit helpers

- [ ] 2.1 `adapters/secondary/http/agent.rs` with `fresh_connections` and `downloads`; transcription, refinement (requests and start-up check), `HttpProbe` (one agent held) and the model fetcher use it; `just test` passes
- [ ] 2.2 `adapters/appkit.rs` with `titled_window`, `below_top`, `label`, `wrapping`, `push_button`; the permissions and download windows use it; `just test` passes and commit

## 3. Menu, settings selectors, handshake, chunking

- [ ] 3.1 Red: unit test that `MenuEntry::place` gives each entry its index in `MENU`; see it fail
- [ ] 3.2 Green: `MenuEntry::place`; `status_item.rs` drops its index constants; the test passes
- [ ] 3.3 Red: unit tests for the settings `Action` table (each action its selector) and `tag` of a non-control is `None`; see them fail
- [ ] 3.4 Green: `Action` enum and selector table in `settings/macos/target.rs`; callers use `wire(control, Action, tag)`; the tests pass
- [ ] 3.5 Red: unit test that the handshake connects through the second address when the first refuses; see it fail
- [ ] 3.6 Green: `handshake` tries every resolved address in `commandspotting/websocket/client.rs`; the test passes
- [ ] 3.7 Drop the no-op `use log;` in `core/application/chunking.rs`; `just test` passes and commit

## 4. Config

- [ ] 4.1 Red: config unit test that a file setting the retired audio key still loads and the loaded audio section carries no such option; see it fail to compile
- [ ] 4.2 Green: the retired key becomes a private ignored slot of `AudioCfg`; the warning in `main.rs` goes; the test passes
- [ ] 4.3 `#[serde(default)]` with each section's `Default` impl replaces the per-field default functions in `config.rs`; the default-file tests pass
- [ ] 4.4 `parse` deserializes once and parses a table only to explain a failure; the removed-section and removed-key tests in `config_loader_it.rs` pass
- [ ] 4.5 `cargo fmt --all`, `just lint`, `just test`; `just size-baseline` and `just analyzers-baseline` drop the entries this change fixed; commit

## 5. Version and manual checks

- [ ] 5.1 Version stays 0.36.1 (no release: refactor)
- [ ] 5.2 Manual checks: none (no hotkey, recording, overlay, paste or permissions behavior changed; the permissions and download windows only share builders)
