# Tasks

## 1. The release reaches the driver

- [x] 1.1 `Session::apply` returns what it produced, and `Session::finish` puts it
      in front of the batch, so a release precedes whatever press follows it
- [x] 1.2 `Session::switch` returns the drag release instead of applying it;
      `switch_to` asserts a session that has not run holds nothing
- [x] 1.3 `Action::Quit` releases the drag before `Effect::Exit`

## 2. Every other way out

- [x] 2.1 `Session::leaving` carries the button a drag is still holding, and `run`
      sends it before `wl.finish()` on the signal, closed-overlay and
      keyboard-gone paths

## 3. The check that it works

- [x] 3.1 `v` then `g` is `[Drag(false), Switch(Grid)]`
- [x] 3.2 `v` then `Escape` is `[Drag(false), Exit]`
- [x] 3.3 A mode change without a drag adds no release, and `leaving()` releases
      exactly once
- [x] 3.4 The headless suite on sway, niri and river: all checks passed

## 4. Documentation and the gate

- [x] 4.1 The `mode-normal` dragging requirement: a mode change ends the drag,
      and the "Moving during a drag" scenario no longer claims it survives one
- [x] 4.2 `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo deny check advisories bans`
