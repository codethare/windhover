# Tasks

## 1. The binding

- [x] 1.1 Point `hint_mode` at `f` in the default table in `src/keys.rs` and in the table of expected defaults in `src/config.rs`. Verify: `cargo test`, including the rebinding test whose "the default is gone" assertion now names the old default.
- [x] 1.2 Nothing else about hint mode moves: the labels, the filtering and the hand-over to normal mode are the same. Verify: the hint unit tests and the hint end-to-end checks pass with only the entry key changed.

## 2. Records

- [x] 2.1 The sample configuration and the options table in `docs/configuration.md`, the mode table and the re-entry note in the README. The sample block is parsed by a test, so it has to stay valid. Verify: `cargo test`.
- [x] 2.2 The end-to-end checks enter hint mode with `f`. Verify: `scripts/headless-check.sh` on sway, river and niri.

## 3. Found while here

- [x] 3.1 The hint specification's purpose still offered targets from "the interactive elements the accessibility tree reports", which went with the `atspi` feature and has been false since. Corrected in the specification itself, since a purpose sentence is prose rather than a requirement and no delta covers it. Verify: `openspec validate --specs`.
