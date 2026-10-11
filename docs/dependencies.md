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
| `swash` | 0.2.10 | glyph rasterization and font metrics | none |
| `rustix` | 1.1.5 | `poll` and `memfd_create` | none |
| `anyhow` | 1.0.104 | error context | RUSTSEC-2026-0190 (`downcast_mut` unsound), fixed in 1.0.103; this project does not use that method |
| `libc` | 0.2 | installing the signal handlers that let a session clean up on `SIGTERM` | none |

The default build's dependency graph is **38** crates (`cargo tree -e normal`).

## Rejected

| crate | why |
|---|---|
| `image`, `imageproc` | several advisories, including CVEs; this project synthesizes pixels from scratch and never decodes an arbitrary image |
| `ttf-parser` (and the wrappers `ab_glyph`, `fontdb`) | RUSTSEC-2026-0192: the author has declared it unmaintained and there is **no** fixed release. A font parser reads a file format, which is exactly the kind of parsing surface worth being careful about |
| `fontique` | it also removes `ttf-parser`, but pulls in ICU, `memmap2` and `hashbrown` for what is here a single step: family name to file path. The system's own `fc-match` does that, with a probe list as the fallback |
| `memmap2` | its advisory is fixed as of 0.9.11, but turning off `xkbcommon`'s default features keeps the mmap path out of the tree entirely |
| `png` | only ever reached through `tiny-skia`'s default feature; nothing here encodes or decodes PNG |
| `clap`, `serde`, `toml` | the command line is four flags and the configuration is a line-oriented `<option>: <value>` file; writing both by hand is about 100 lines, less than the transitive weight of those crates |

## Dependencies of the opt-in `atspi` feature

| crate | used for |
|---|---|
| `atspi` 0.30 (`connection` + `proxies` + `zbus`, without `p2p`/`wrappers`/`tracing`) | reading the accessibility tree |
| `futures-executor` 0.3 | the AT-SPI bindings are asynchronous; `block_on` connects them to a single-threaded session |

## What the `atspi` feature costs and buys

`--features atspi` grows the graph from **38** crates to **103** (`atspi` →
`zbus` → the async-io ecosystem). It is off by default, so only builds that ask
for it are affected.

What it buys: hint targets come from the centres of the interactive elements in
the accessibility tree (buttons, entries, list items, links and so on) rather
than only from a regular lattice. When the accessibility bus is unavailable, when
there are no candidates, or when the walk exceeds its time budget, hint mode
falls back to the lattice silently — so **on a system without an accessibility
bus, and for an application that does not publish a tree, hint mode behaves
exactly as the default build does.** `scripts/headless-check.sh` runs with
`FEATURES=atspi` and asserts the same landing positions.

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
