# Tasks

## 1. Report the bindings the layout cannot produce

- [x] 1.1 `src/wl/seat.rs` enumerates every keysym the installed keymap can produce, over all of its layouts and levels, and reports each keymap it installs. Verify: the unit test uses the test keymap and checks that a key it gives no symbol to is not reported as reachable.
- [x] 1.2 `src/config.rs` answers which configured bindings are missing from that set, named as the options they were set by, with the grid cell keys named as `grid_cells`. Verify: `cargo test` covers a layout missing one binding, a complete layout, and the cell keys.
- [x] 1.3 `src/session.rs` asks on every keymap install, so a layout switch is covered, and writes one line naming them. Verify: the end-to-end check configures a binding a real layout cannot produce and expects the line; removing the line makes it fail.
- [x] 1.4 The check has to stay silent when nothing is missing, or it is noise on every session: the existing "sessions stayed quiet" check covers that on all three compositors.

## 2. Trace

- [x] 2.1 `src/trace.rs` holds the flag and the one-line writer, with the formatting left lazy so an untraced session pays nothing.
- [x] 2.2 `-t` / `--trace` in `src/main.rs`, kept out of the mode flags: it combines with any of them. Verify: the argument test reads it, keeps it with the mode, and combines it with `--oneshot`.
- [x] 2.3 Instrument what a bug report needs: focus changes, keys with their keysyms, keymap installs, mode changes, and a loop turn line when there was an effect or no time to wait. Verify: the end-to-end check runs a traced session and expects all three.
- [x] 2.4 A session without the flag writes nothing extra. Verify: the quiet check.

## 3. Records

- [x] 3.1 README: what the trace is for and how to include it in a report. Verify: the flags in `--help` and the README agree.
