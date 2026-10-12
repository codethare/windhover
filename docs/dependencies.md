# Dependency admission and audit

This file is windhover's record of which dependencies are allowed in. Any new
direct dependency has to be registered here first and pass the check below.

## Admission rules

1. **No unfixed RustSec advisory.** The `informational` category counts too:
   `unmaintained` is a straight rejection, because a parser nobody maintains
   means future vulnerabilities nobody fixes.
2. **No public security incident.** Before adding a crate, look for its CVEs,
   GitHub security advisories and any incident write-ups.
3. **Prefer pure Rust.** When a C library is genuinely needed, only ones that
   are already deployed widely on the target platform are allowed (currently
   `libxkbcommon` alone).
4. **Anything a crate would be pulled in for "convenience" is written by hand
   instead.** That is why there is no `clap`, `serde` or `toml`.
5. **A dependency that can be switched off must not be in the default build.**

## Direct dependencies of the default build

| crate | locked version | used for | advisory |
|---|---|---|---|
| `wayland-client` | 0.31.15 | the Wayland connection and its event queue | none |
| `wayland-protocols` | 0.32.12 | `wp_fractional_scale_v1`, `wp_viewporter` | none |
| `wayland-protocols-wlr` | 0.3.12 | `zwlr_layer_shell_v1`, `zwlr_virtual_pointer_v1` | none |
| `xkbcommon` | 0.9.0 | key symbol resolution (bindings to the system `libxkbcommon`); `default-features = false` to avoid its `memmap2`-based fd convenience API | none |
| `tiny-skia` | 0.12.0 | software rasterization: rounded corners, strokes, antialiasing; `default-features = false` to drop the PNG codec we never call | none |
| `skrifa` | 0.44.0 | glyph outlines and font metrics. The outlines are filled with `tiny-skia`, so there is no second rasterizer in the tree | none |
| `rustix` | 1.1.5 | `poll` and `memfd_create` | none |
| `anyhow` | 1.0.104 | error context | RUSTSEC-2026-0190 (`downcast_mut` unsound), fixed in 1.0.103; this project does not use that method |
| `libc` | 0.2 | installing the signal handlers that let a session clean up on `SIGTERM` | none |

The default build's dependency graph is **38** crates for the Linux target
(`cargo tree -e normal --target x86_64-unknown-linux-gnu`), or 45 (name, version)
pairs across all platforms.

The crate's declared `rust-version` is **1.85**, which is what edition 2024
requires. It was raised to 1.87 briefly, to take a `zbus_xml` that would build
without the vulnerable `quick-xml`; dropping the `atspi` feature removed that
reason along with the whole subgraph, so it is back down.

## Rejected

| crate | why |
|---|---|
| `image`, `imageproc` | several advisories, including CVEs; this project synthesizes pixels from scratch and never decodes an arbitrary image |
| `ttf-parser` (and the wrappers `ab_glyph`, `fontdb`) | RUSTSEC-2026-0192: the author has declared it unmaintained and there is **no** fixed release. A font parser reads a file format, which is exactly the kind of parsing surface worth being careful about |
| `fontique` | it also removes `ttf-parser`, but pulls in ICU, `memmap2` and `hashbrown` for what is here a single step: family name to file path. The system's own `fc-match` does that, with a probe list as the fallback |
| `swash` (with `yazi` and `zeno`) | reached `skrifa`/`read-fonts` for parsing, but carried its own scaling, metrics and shaping layer whose upstream is dormant and has open panic reports (`Bitmap::decode`, `GlyphMetrics::advance_width`, and overflow panics in `parse.rs`, `metrics.rs` and `scale/mod.rs`). Since `tiny-skia` was already there to fill a path, drawing the outlines from `skrifa` directly removes three crates and the unmaintained layer without losing anything |
| `memmap2` | its advisory is fixed as of 0.9.11, but turning off `xkbcommon`'s default features keeps the mmap path out of the tree entirely |
| `png` | only ever reached through `tiny-skia`'s default feature; nothing here encodes or decodes PNG |
| `atspi` (with `zbus`, `futures-executor` and 64 more) | 67 crates — more than the rest of the project combined — for one optional capability, element-level hint targets. It was never verified: the test machine has no accessibility bus and no accessible application, so only its fallback path was measurable. It was also the origin of the project's one advisory (`quick-xml`, through `zbus`) and the only part of the tree opening IPC to another process. Hint mode uses the lattice, which was already its default source |
| `clap`, `serde`, `toml` | the command line is four flags and the configuration is a line-oriented `<option>: <value>` file; writing both by hand is about 100 lines, less than the transitive weight of those crates |

## Code that runs at build time

`build.rs` scripts and proc macros execute with the developer's privileges
during `cargo build`, so they are dependencies that run code and are inventoried
here rather than discovered later.

The build has **8 crates with a build script** — `anyhow`, `libc`,
`proc-macro2`, `quote`, `rustix`, `wayland-backend`, `wayland-client`,
`wayland-sys` — and **2 proc-macro crates**: `bytemuck_derive` and
`wayland-scanner`. (Before the `atspi` feature was dropped those were 17 and 13.)
Every one of them comes from crates.io with a checksum in `Cargo.lock`, which is
what makes "no `git = ` or `path = ` dependencies" worth enforcing: it is the
only thing anchoring those scripts to a reviewed artefact.

## What the gate caught

The gate had a bug, and the bug hid a real advisory. It read the graph into a
mapping keyed by crate name, so when two versions of one crate were present only
one of them was ever checked. `zbus` (through the `atspi` feature) pulls
`quick-xml` 0.38.4 while `wayland-scanner` pulls 0.41.0, and the surviving entry
happened to be the patched one:

```
quick-xml 0.38.4: RUSTSEC-2026-0194   (quadratic run time on duplicate attributes)
quick-xml 0.38.4: RUSTSEC-2026-0195   (unbounded namespace allocation)
```

Both are denial of service (`A:H`, no confidentiality or integrity impact), and
both are fixed in `quick-xml` 0.41.0. The default build was never affected: only
0.41.0 is in its graph. The `atspi` build was.

The fix, in two parts:

- `scripts/audit-deps.py` now keeps a set of `(name, version)` pairs instead of a
  name-keyed mapping, so a crate at two versions is checked at both.
- `zbus_xml` 5.2.1 dropped `quick-xml` for `winnow`, so raising `rust-version` to
  `1.87` and running `cargo update` removed `quick-xml` 0.38.4 from the graph.
  `quick-xml` 0.41.0 (through `wayland-scanner`) stays and is patched.
- The `atspi` feature was then dropped outright (see the rejected table), which
  removed `zbus` and its 65 crates rather than tracking them, and let the MSRV go
  back to 1.85.

## The check

```sh
python3 scripts/audit-deps.py
```

The script:

1. clones the RustSec database into `$RUSTSEC_DB` (default `/tmp/advisory-db`)
   when it is not already there;
2. reads the whole graph, opt-in features included, with
   `cargo metadata --all-features`;
3. compares every package against the `patched` and `unaffected` ranges in
   `advisory-db/crates/<name>/RUSTSEC-*.md`;
4. lists any advisory no patched release covers, and exits non-zero.

`--all-features` is deliberate: a transitive dependency that only an opt-in
feature brings in is still in the gate.
