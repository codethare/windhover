# windhover — project rules for agents

## Dependencies are held to a strict security standard

No Cargo dependency is added, removed or bumped without passing every rule
below. This applies to transitive dependencies too: a crate that arrives
indirectly is a dependency of this project whether or not it is named in
`Cargo.toml`.

1. **No unfixed RustSec advisory.** The `informational` category counts.
   `unmaintained` is a rejection, not a warning: a crate nobody maintains is a
   vulnerability nobody will fix. Check the whole graph, opt-in features
   included.
2. **No public security incident.** Before adding a crate, look for its CVEs,
   its GitHub security advisories, and write-ups of any incident involving it. A
   crate that has had a supply-chain compromise does not go in.
3. **crates.io only.** No `git = ` and no `path = ` dependencies: they bypass
   the registry's checksums, and `Cargo.lock` cannot pin them to a reviewed
   artefact.
4. **Registered in [`docs/dependencies.md`](docs/dependencies.md)** with its
   version, what it is for, and its advisory status — before the code that uses
   it lands. If a candidate is rejected, record why there too, so the same
   evaluation is not repeated.
5. **Pure Rust preferred.** A C library needs a reason, and is limited to what
   the target platform already ships. Today that is `libxkbcommon` alone, and it
   is the only non-libc library the binary links.
6. **Convenience is not a reason.** If a crate would be pulled in to save a
   hundred lines, write the hundred lines. That is why there is no `clap`,
   `serde` or `toml` here.
7. **A dependency that can be switched off stays off by default.** It belongs
   behind a feature, and the default build must not carry it.

### The gate

```sh
python3 scripts/audit-deps.py
```

It resolves the graph with `cargo metadata --all-features`, compares every
package against the RustSec advisory database, and exits non-zero if any
advisory is not covered by a patched release. A change is not finished while
this exits non-zero.

### Compile-time execution counts as risk

`build.rs` scripts and proc macros run with the developer's privileges during
`cargo build`. They are dependencies that execute code, so their number and
identity matter as much as any runtime dependency: both are recorded in
`docs/dependencies.md`, and a new one is a reason to look twice before accepting
it.

### What "strictly controlled" bought

It has already changed the dependency set twice, and both times the answer was
"take it out":

- `ab_glyph`/`fontdb` were rejected because their underlying `ttf-parser` is
  declared unmaintained with no fixed release (RUSTSEC-2026-0192), and the font
  work moved to `swash` with the system's own `fc-match` for family resolution.
- The `atspi` feature was dropped after it proved to be 67 crates — more than the
  rest of the project combined — for one optional capability that the test
  machine could not even measure, and the origin of the project's only advisory.
  The rule that a dependency which can be switched off stays off by default was
  not enough on its own: a feature nobody builds is a feature nobody audits.

Expect the same answer for the next crate that only looks convenient.
