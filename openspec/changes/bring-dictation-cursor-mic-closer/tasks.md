## 1. Sign closer to the pointer

- [ ] 1.1 Red: change the placement unit tests in `app/src/adapters/secondary/visualfeedback/macos/pointer_sign.rs` to expect the sign 8 pt right of and 6 pt below the pointer; run them and see them fail
- [ ] 1.2 Green: split the gap into a right offset of 8 pt and a down offset of 6 pt in `app/src/adapters/secondary/visualfeedback/macos/pointer_sign.rs`
- [ ] 1.3 `just test` and commit

## 2. Release

- [ ] 2.1 Bump `version` in `app/Cargo.toml` to 0.21.1
- [ ] 2.2 `just test`, `just check` once, and commit
- [ ] 2.3 Manual checks by the user: start a recording, see the 🎙️ sign sit right at the pointer's tip, moving with it
