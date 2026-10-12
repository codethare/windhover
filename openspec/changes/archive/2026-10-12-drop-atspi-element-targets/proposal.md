# Proposal

## Why

The `atspi` feature costs 67 crates — more than everything else in the project
put together — and buys one optional capability: element-level hint targets. It
is off by default, so the default build never exercises it, and the verification
suite never measured it either: the machine used for testing has no accessibility
bus and no accessible application, so only the fallback path was ever observed.
It is also the origin of the one advisory this project has had to fix
(`quick-xml`, reached through `zbus`), and the only part of the tree that opens
IPC to another process. Hint mode does not depend on it: the lattice is the
default source and the fallback.

## What Changes

- Remove the `atspi` cargo feature, the `atspi` and `futures-executor`
  dependencies, and `src/hint/atspi.rs`.
- Collapse the `HintSource` trait, `Cascade` and `select()` — which would have a
  single implementation left — down to the lattice itself.
- Drop the `atspi_budget` configuration value and the plumbing that carried it.
- Update the README, `docs/dependencies.md` and `docs/compositors.md`, and remove
  the `FEATURES=atspi` case from the end-to-end script, which no longer has a
  feature to pass.
- Dependency graph: from 104 crates with all features to 37, build scripts from
  17 to 8, proc macros from 13 to 2. The default build is unchanged.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `mode-hint`: the "Element targets and degradation" requirement is removed,
  because the capability it describes no longer exists.

## Impact

- Code: `src/hint/atspi.rs` deleted; `src/hint/source.rs`, `src/hint/mod.rs`,
  `src/config.rs` and `src/session.rs` simplified.
- Dependencies removed: `atspi`, `futures-executor`, and 65 transitive crates.
- No user-visible behaviour change for anyone who was not building with
  `--features atspi`: hint mode already used the lattice by default, and on a
  system without an accessibility bus it used the lattice there too.
- Anyone who *was* building with `--features atspi` loses element-level targets
  and keeps everything else.
