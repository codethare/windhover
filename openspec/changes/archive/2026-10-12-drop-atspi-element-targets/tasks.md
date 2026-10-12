# Tasks

## 1. Remove the feature

- [x] 1.1 Drop the `atspi` and `futures-executor` dependencies and the `[features]` section from `Cargo.toml`, then delete `src/hint/atspi.rs` and its `pub mod` from `src/hint/mod.rs`. Verify: `cargo build` and `cargo tree -e normal` no longer contain `atspi`, `zbus` or `futures-executor`, and the graph is 9 direct dependencies.
- [x] 1.2 Collapse `src/hint/source.rs`: remove the `HintSource` trait, `Cascade` and `select()`, leaving `Lattice::targets` as the only source. Verify: `Lattice`'s unit tests still cover the target geometry and the density behaviour.
- [x] 1.3 Remove `atspi_budget` from `src/config.rs` and the call site in `src/session.rs`. Verify: `cargo test` passes with no test referring to the budget.
- [x] 1.4 Remove the `FEATURES` pass-through from `scripts/headless-check.sh`. Verify: `sh -n scripts/headless-check.sh` is clean and a run still passes.

## 2. Update the records

- [x] 2.1 Remove the accessibility sections from `README.md`, `docs/dependencies.md` and `docs/compositors.md`, and replace the crate counts in `docs/dependencies.md` with the new ones. Verify: `grep -ri atspi` finds nothing outside the archived change, and the counts match `cargo tree`.
- [x] 2.2 Record the removal in `CLAUDE.md`'s "what strictly controlled bought" section, alongside the `ttf-parser` rejection, so the same evaluation is not repeated. Verify: the section names both rejections.

## 3. Verify

- [x] 3.1 Run the audit and the whole test suite: `python3 scripts/audit-deps.py`, `cargo test`. Verify: the audit exits 0 over the smaller graph and every test passes.
- [x] 3.2 Run the end-to-end suite on all three compositors. Verify: sway 26/26, river 26/26, niri 25/26 (the accessibility-independent check that niri always advertises a keyboard for), no failures, and hint mode still lands on its first lattice target.
- [x] 3.3 Archive the change and confirm `openspec/specs/mode-hint/spec.md` no longer contains the removed requirement. Verify: `openspec list --specs` shows `mode-hint` with one fewer requirement, and `openspec validate --strict` passes.
