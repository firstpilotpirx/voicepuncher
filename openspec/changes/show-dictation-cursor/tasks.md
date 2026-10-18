# Tasks

## 1. Dictation sign follows the overlay's recording signal

- [ ] 1.1 Red: add `app/tests/features/component/dictation_pointer.feature` with the seven component scenarios of the spec, steps in `app/tests/steps/dictation_pointer.rs` (real pipeline, fake sign view through `PointerSignIndicator`); run and see it fail
- [ ] 1.2 Green: add `PointerSignView` and `PointerSignIndicator` in `app/src/adapters/secondary/visualfeedback/pointer_sign.rs`; show on recording, hide on every other signal
- [ ] 1.3 Add the term "dictation sign" to `docs/ubiquitous-language.md`
- [ ] 1.4 `just test`, commit

## 2. macOS dictation sign window

- [ ] 2.1 Red: unit test the sign's origin from the pointer location and size in `app/src/adapters/secondary/visualfeedback/macos/pointer_sign.rs`; see it fail
- [ ] 2.2 Green: implement the placement function
- [ ] 2.3 Red: adapter test in `app/tests/visualfeedback_macos_it.rs`: the sign window ignores mouse events and never becomes key; see it fail
- [ ] 2.4 Green: implement `MacosPointerSign` (borderless 🎙️ window, pop-up menu level, all Spaces, 16 ms follow timer while shown)
- [ ] 2.5 Wire `PointerSignIndicator` around `MacosVisualIndicator` with `MacosPointerSign` in `app/src/main.rs`
- [ ] 2.6 `just test`, commit

## 3. Version and checks

- [ ] 3.1 Bump `version` in `app/Cargo.toml` to 0.16.0
- [ ] 3.2 `just check` once, commit
- [ ] 3.3 Manual run by the user: the sign follows the pointer during a hotkey and a voice recording, crosses to a second screen, lets clicks through, and is gone after stop, cancel and quitting during a recording
